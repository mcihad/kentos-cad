//! The source line under Öznitelik tablosu's bar (docs/adr/0546 §3; plan/gis.md CBS-40): for a layer bound to a
//! GeoPackage or a PostGIS table, the table's state in Kaynak durumu's words (the source, what waits, how much of
//! the table the layer holds) and the one button the state asks for. The state is `cad.sources.describe`'s
//! (`sources::layer_state`), the button's event `source_status::way_out`: neither rule is written again here.
//!
//! Reading the state reads the table, so it is kept: read again when the layer shown, its binding (its digest and
//! revision, which a write or a read changes), the drawing's session or Yenile changed, and, on a layer of at most
//! [`LIVE_MOST`] objects, whenever the drawing changed (a layer edit is “Katmanda yazılmamış” at once). A larger
//! layer keeps the last reading and says so (“Yenile”) instead of hashing a million objects after every edit.

use iced::widget::{button, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{SourceNext, SourceState};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Elided;
use kentos_ui::{label, style};

use super::Rows;
use crate::app::{App, Message};
use crate::source_status::{
    act_words, scope_words, source_words, state_words, system_words, way_out,
};

/// A layer of at most this many objects is read again after every change of the drawing.
pub(crate) const LIVE_MOST: usize = 20_000;

const SOURCE: &str = "Kaynak";
const REFRESH: &str = "Yenile";
const REFRESH_HINT: &str = "Kaynağı yeniden okur (dosya dışarıdan değişmiş olabilir)";
const READ_ONLY: &str = "Salt okunur";
const STALE: &str = "Katman değişti; durum eski";

/// The binding's state as last read, and what it was read under.
pub(crate) struct SourceLine {
    key: Key,
    /// The drawing's generation at the reading.
    generation: u64,
    pub state: Option<SourceState>,
}

/// The drawing's session, the layer, its binding's digest and revision, Yenile's count, and the drawing's
/// generation (kept 0 on a layer too large to follow it).
type Key = (u64, String, String, String, u64, u64);

/// Messages of the line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceEvent {
    /// The state's way out.
    Act,
    Refresh,
}

impl App {
    /// The state of the table's layer's binding, read when it must be; none for a layer that is not bound. The
    /// flag: the reading is of an earlier drawing (a layer too large to follow it).
    pub(crate) fn source_state(&self, rows: &Rows) -> Option<(SourceState, bool)> {
        let doc = self.document.as_ref()?;
        let layer = rows.layer.as_deref()?;
        let link = doc
            .model
            .settings()
            .sources
            .iter()
            .find(|l| l.layer == layer)?;
        let live = rows.slots.len() <= LIVE_MOST;
        let generation = doc.model.generation();
        let key: Key = (
            doc.session,
            layer.to_owned(),
            link.digest.clone(),
            link.revision.clone(),
            self.features.source_refresh,
            if live { generation } else { 0 },
        );
        let mut held = self.features.source.borrow_mut();
        if held.as_ref().is_none_or(|h| h.key != key) {
            *held = Some(SourceLine {
                key,
                generation,
                state: kentos_native_application::sources::layer_state(&doc.model, layer),
            });
        }
        let line = held.as_ref()?;
        Some((line.state.clone()?, line.generation != generation))
    }

    pub(crate) fn features_source_event(&mut self, event: SourceEvent) -> Task<Message> {
        match event {
            SourceEvent::Refresh => {
                self.features.source_refresh += 1;
                Task::none()
            }
            SourceEvent::Act => {
                let Some(doc) = &self.document else {
                    return Task::none();
                };
                let rows = self.feature_rows(doc);
                let Some((state, _)) = self.source_state(&rows) else {
                    return Task::none();
                };
                // A read of the table says what waits now, not what was shown a moment ago.
                let Some(event) = way_out(state.next, Some(state.link.layer.clone())) else {
                    return Task::none();
                };
                let task = self.bound_event(event);
                self.features.source_refresh += 1;
                self.features.changed();
                task
            }
        }
    }

    /// The source line over the table; none for a layer that is not bound.
    pub(crate) fn features_source_line<'a>(&self, rows: &Rows) -> Option<Element<'a, Message>> {
        let (state, stale) = self.source_state(rows)?;
        let srid = self
            .document
            .as_ref()
            .map_or(0, |d| d.model.settings().srid);
        // The four scopes apart (plan/gis.md CBS-20): the table's rows in the layer (scope_words: the layer's
        // objects and rows of the table), the ones shown after the filter, the selected, and the whole table.
        let selected = self.document.as_ref().map_or(0, |d| {
            self.selection
                .ids()
                .iter()
                .filter(|s| {
                    d.model
                        .get(**s)
                        .is_some_and(|e| e.base().layer_id == state.link.layer)
                })
                .count()
        });
        let words = format!(
            "{} · {} · {} · {} gösterilen · {selected} seçili · {}",
            source_words(&state),
            state_words(&state),
            scope_words(&state),
            rows.shown.len(),
            system_words(&state, srid)
        );
        let act: Option<Element<'a, Message>> = act_words(state.next, state.link.kind).map(|w| {
            let blocked = state.next == SourceNext::Write && !state.link.writable;
            button(label::caption(if blocked { READ_ONLY } else { w }))
                .on_press_maybe(
                    (!blocked).then(|| Message::Features(super::Event::Source(SourceEvent::Act))),
                )
                .padding([2, 8])
                .style(style::button::secondary)
                .into()
        });
        let note: Option<Element<'a, Message>> = stale.then(|| {
            label::caption(STALE)
                .style(|t: &iced::Theme| iced::widget::text::Style {
                    color: Some(Tokens::of(t).warning),
                })
                .into()
        });
        let mut line = row![
            container(label::caption(SOURCE).style(style::text::muted)).padding([0, 2]),
            Elided::new(words)
                .size(typography::caption())
                .font(typography::ui())
                .style(|t: &iced::Theme| iced::widget::text::Style {
                    color: Some(Tokens::of(t).text),
                })
                .width(Fill),
        ]
        .spacing(8)
        .align_y(Center);
        if let Some(n) = note {
            line = line.push(n);
        }
        if let Some(a) = act {
            line = line.push(a);
        }
        line = line.push(self.whole_switch(&state.link.layer));
        line = line.push(kentos_ui::widget::tip(
            button(label::caption(REFRESH))
                .on_press(Message::Features(super::Event::Source(
                    SourceEvent::Refresh,
                )))
                .padding([2, 8])
                .style(style::button::subtle),
            kentos_ui::widget::Tip::new(REFRESH).body(REFRESH_HINT),
            iced::widget::tooltip::Position::Top,
        ));
        Some(container(line).padding([2, 10]).width(Fill).into())
    }
}
