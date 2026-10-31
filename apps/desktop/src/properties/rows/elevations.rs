//! A derived drawing's section (docs/adr/0799 §5): the object is drawn in a room's interior elevation, made from the
//! model by `cad.elevations.edit`. The section says it is derived (not a mark, not hand drawn), from which space and
//! which sides, whether drawing it again would differ (Güncel, Eski, Kaynağı yok) and how many of its objects are
//! missing or changed by hand; Yeniden üret and Bağı kopar write through `cad.elevations.edit` in one undo step each.
use kentos_commands::{ElevationState, ElevationsOperation};
use kentos_domain::Slot;

use super::{Choice, Editor, Event, Row, Section};
use crate::app::Message;
use crate::document::Document;

/// The derived drawing section of the object in `slot`; none when its layer holds no interior elevation.
pub(super) fn section(doc: &Document, slot: Slot) -> Option<Section> {
    let e = doc.model.get(slot)?;
    let layer = e.base().layer_id.clone();
    let uid = doc.model.uid(slot)?.to_string();
    let (_, record) = kentos_native_application::elevations::on_layer(&doc.model, &layer)?;
    // Only the elevation's own objects: one drawn by hand on its layer is not derived.
    if !record.members.values().any(|m| *m == uid) {
        return None;
    }
    let row = kentos_native_application::elevations_query::row(&doc.model, &layer)?;
    let locked = doc.model.layers().is_locked(&layer);
    let mut rows = vec![Row::text("Tür", "Türetilmiş çizim · oda iç görünüşü")];
    rows.push(Row::text(
        "Kaynak",
        row.space_label
            .clone()
            .map_or_else(|| "Mahal (silinmiş)".to_owned(), |l| format!("{l} (mahal)")),
    ));
    let sides = if row.sides.is_empty() {
        "Bütün düz yanlar".to_owned()
    } else {
        row.sides
            .iter()
            .map(|k| kentos_native_application::elevations::letter(*k))
            .collect::<Vec<_>>()
            .join(", ")
    };
    rows.push(Row::text("Yanlar", sides));
    let mut state = match row.state {
        ElevationState::Current => "Güncel".to_owned(),
        ElevationState::Stale => "Eski: model değişti, yeniden üretin".to_owned(),
        ElevationState::Gone => "Kaynağı yok: mahal silinmiş".to_owned(),
    };
    if !row.missing.is_empty() {
        state.push_str(&format!(" · {} nesne silinmiş", row.missing.len()));
    }
    if !row.edited.is_empty() {
        state.push_str(&format!(" · {} nesne elle değişmiş", row.edited.len()));
    }
    let action = |label: &'static str, icon: &'static str, op: Option<ElevationsOperation>| Choice::Action {
        label,
        icon,
        message: op
            .filter(|_| !locked)
            .map(|op| Message::Properties(Event::Elevation(op))),
    };
    let refresh = (row.state != ElevationState::Gone
        && (row.state == ElevationState::Stale || !row.edited.is_empty()))
    .then(|| ElevationsOperation::Refresh {
        layer: layer.clone(),
    });
    let items = vec![
        action("Yeniden üret", "roomElevation", refresh),
        Choice::Separator,
        action(
            "Bağı kopar (çizim kalır)",
            "moduleDetach",
            Some(ElevationsOperation::Release {
                layer: layer.clone(),
            }),
        ),
    ];
    rows.push(Row::text("Durum", state.clone()).editor(Some(Editor::Select {
        text: state,
        swatch: None,
        icon: None,
        items,
    })));
    rows.push(Row::text("Türetildiği sürüm", row.derived_at.clone()));
    Some(Section {
        id: "elevation",
        title: format!("İç görünüş: {}", row.name).into(),
        rows,
    })
}
