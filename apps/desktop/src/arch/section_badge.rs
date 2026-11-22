//! Eski kesit rozeti (docs/adr/8042 §5): while a section drawn from a section mark is no longer what the model would
//! draw (a door moved, a storey grew), the status bar says so in the danger colour; clicked, the section's placement is
//! selected and Öznitelikler offers Yeniden çiz. An elevation drawn from an elevation mark (docs/adr/8043) the same:
//! “Kuzey görünüşü eski”.
//!
//! Each section is drawn again to compare: that is worked out once for each change of the drawing, not every frame,
//! and kept by the opened drawing (its session) and its generation (docs/adr/8140).

use std::cell::RefCell;

use iced::Element;
use iced::widget::text;
use kentos_commands::BuildingSectionState;
use kentos_ui::label;
use kentos_ui::theme::Tokens;
use kentos_ui::widget::Tip;
use kentos_ui::widget::status_bar::Readout;

use crate::app::{App, Message};
use crate::document::Document;

/// A section as the badge and Öznitelikler read it.
#[derive(Clone, Debug, PartialEq)]
pub struct Info {
    /// Its mark's persistent id; none when the mark is gone.
    pub mark: Option<String>,
    /// "A-A kesiti", "Kuzey görünüşü".
    pub name: String,
    /// An elevation's (Görünüş çıkar), not a section's.
    pub facade: bool,
    pub state: BuildingSectionState,
    /// Its block's id, when drawn.
    pub block: Option<String>,
    /// Its first placement's persistent id.
    pub first: Option<String>,
}

thread_local! {
    /// The last reading: the opened drawing (its session) and its generation, and its sections.
    #[allow(clippy::type_complexity)]
    static LAST: RefCell<Option<((u64, u64), Vec<Info>)>> = const { RefCell::new(None) };
}

/// The drawing's sections and their states, read again only when the drawing is another or changed: drawing a section
/// again to compare is not a frame's work.
pub fn sections(opened: &Document) -> Vec<Info> {
    let key = (opened.session, opened.model.generation());
    let doc = &opened.model;
    LAST.with(|last| {
        let mut last = last.borrow_mut();
        match &*last {
            Some((at, r)) if *at == key => r.clone(),
            _ => {
                let rows: Vec<Info> = kentos_native_application::sections_query::rows(doc)
                    .into_iter()
                    .map(|r| {
                        let first = r.mark.as_deref().and_then(|m| {
                            let u = kentos_domain::Uuid::parse_str(m).ok()?;
                            kentos_native_application::sections_edit::inserts_of(doc, u)
                                .into_iter()
                                .next()
                        });
                        let facade = r.mark.as_deref().map_or_else(
                            || r.name.starts_with("MM-XXXX-GORUNUS"),
                            |m| {
                                kentos_domain::Uuid::parse_str(m)
                                    .ok()
                                    .and_then(|u| doc.slot_of(u))
                                    .and_then(|s| doc.get(s))
                                    .is_some_and(|e| {
                                        matches!(e, kentos_commands::Entity::ViewMark(v)
                                            if v.mark.form == kentos_commands::ViewMarkForm::Elevation)
                                    })
                            },
                        );
                        Info {
                            mark: r.mark,
                            name: r.name,
                            facade,
                            state: r.state,
                            block: r.block,
                            first,
                        }
                    })
                    .collect();
                *last = Some((key, rows.clone()));
                rows
            }
        }
    })
}

/// The status bar's cell while a section or an elevation is stale: its name ("A-A kesiti eski"), or how many of each
/// ("2 kesit, 1 görünüş eski"); none while every one is current.
pub fn cell(app: &App) -> Option<Element<'_, Message>> {
    let doc = app.document.as_ref()?;
    let all = sections(doc);
    let old: Vec<&Info> = all
        .iter()
        .filter(|i| i.state == BuildingSectionState::Stale)
        .collect();
    let first = old.first()?;
    let facades = old.iter().filter(|i| i.facade).count();
    let cuts = old.len() - facades;
    let words = match (cuts, facades) {
        _ if old.len() == 1 => format!("{} eski", first.name),
        (n, 0) => format!("{n} kesit eski"),
        (0, n) => format!("{n} görünüş eski"),
        (c, f) => format!("{c} kesit, {f} görünüş eski"),
    };
    let (title, these) = match (cuts, facades) {
        (_, 0) => ("Eski kesit", "bu kesitler"),
        (0, _) => ("Eski görünüş", "bu görünüşler"),
        _ => ("Eski kesit ve görünüş", "bu kesit ve görünüşler"),
    };
    let body = format!(
        "Plan değişti; {these} artık modelin çizeceği gibi değil: {}. Tıklayın: ilki seçilir, Öznitelikler'de \
         Durum › Yeniden çiz.",
        old.iter()
            .map(|i| i.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let press = first.first.clone().map(Message::SelectUid);
    let mut cell =
        Readout::new(
            label::strong(words).style(move |theme: &iced::Theme| text::Style {
                color: Some(Tokens::of(theme).danger),
            }),
        )
        .icon(crate::icons::from_web(Some(if cuts == 0 {
            "facadeDraw"
        } else {
            "sectionDraw"
        })))
        .tip(Tip::new(title).body(body));
    if let Some(m) = press {
        cell = cell.on_press(m);
    }
    Some(cell.into())
}
