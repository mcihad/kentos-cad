//! A section's rows (docs/adr/8042 §5): the object is a section's placement (its mark's block) or a section mark. The
//! rows say which section it is, whether drawing it again would differ (Güncel, Eski, Çıkarılmadı) and offer Yeniden
//! çiz through `cad.sections.edit`, one undo step. Eski is said in the danger colour, as BricsCAD marks a drawing out of
//! sync red.
use kentos_commands::{BuildingSectionState, BuildingSectionsOperation, Entity, ViewMarkForm};
use kentos_domain::Slot;

use super::{Choice, Editor, Event, Row, Section};
use crate::app::Message;
use crate::document::Document;

/// The section rows of the object in `slot`: a section's placement or a section mark; none for any other.
pub(super) fn section(doc: &Document, slot: Slot) -> Option<Section> {
    let e = doc.model.get(slot)?;
    let (mark_uid, mark) = match e {
        Entity::Insert(i) => doc.model.entities_with_uids().find_map(|(u, x)| match x {
            Entity::ViewMark(v)
                if v.mark.form == ViewMarkForm::Section
                    && kentos_native_application::sections::block_id(u) == i.block =>
            {
                Some((u, v.mark.clone()))
            }
            _ => None,
        })?,
        Entity::ViewMark(v) if v.mark.form == ViewMarkForm::Section => {
            (doc.model.uid(slot)?, v.mark.clone())
        }
        _ => return None,
    };
    // The badge's reading, once per revision: drawing the section again to compare is not a frame's work.
    let state = crate::section_badge::sections(&doc.model)
        .into_iter()
        .find(|i| i.mark.as_deref() == Some(mark_uid.to_string().as_str()))
        .map_or(BuildingSectionState::NotDrawn, |i| i.state);
    let name = format!(
        "{} kesiti",
        kentos_native_application::sections::label(&mark.mark)
    );
    let mut rows = vec![Row::text("Tür", "Kesit · plandan çıkarılır (1/50)")];
    let words = match state {
        BuildingSectionState::Current => "Güncel".to_owned(),
        BuildingSectionState::Stale => "Eski: plan değişti, yeniden çizin".to_owned(),
        BuildingSectionState::NotDrawn => "Çıkarılmadı: Kesit çıkar ile çizin".to_owned(),
        BuildingSectionState::Gone => "Kaynağı yok: kesit işareti silinmiş".to_owned(),
    };
    let refresh =
        (state == BuildingSectionState::Stale).then(|| BuildingSectionsOperation::Refresh {
            mark: mark_uid.to_string(),
        });
    let items = vec![Choice::Action {
        label: "Yeniden çiz",
        icon: "sectionDraw",
        message: refresh.map(|op| Message::Properties(Event::Section(op))),
    }];
    rows.push(
        Row::text("Durum", words.clone()).editor(Some(Editor::Select {
            text: words,
            swatch: None,
            icon: None,
            items,
        })),
    );
    Some(Section {
        id: "section".into(),
        title: name.into(),
        rows,
    })
}
