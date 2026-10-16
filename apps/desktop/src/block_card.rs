//! Blok kartı (docs/adr/0317 §4): a block's library card, its category (a path by `/`), the words it is found by
//! and its description, written with `cad.blocks.edit`'s `card` as one undo step “Blok değiştir”. Its version is the
//! drawing's count of the changes to what it draws; the window shows it and its size, unit and attributes.

use iced::widget::{Column, text, text_input};
use iced::{Element, Task};
use kentos_contracts::{BlockEditOperation, BlockId, BlocksEdit, CommandResult};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, blocks_edit};
use kentos_ui::style;
use kentos_ui::widget::{Dialog as Frame, focus_ring, overlay};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};

/// The window's title, which a trace names it by.
pub const CARD_TITLE: &str = "Blok kartı";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";

#[derive(Debug, Clone)]
pub struct Window {
    block: BlockId,
    name: String,
    category: String,
    tags: String,
    description: String,
    /// What it says beside the fields: its version, size, unit and attributes.
    about: Vec<String>,
    /// The command's refusal, after Kaydet.
    refused: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Category(String),
    Tags(String),
    Description(String),
    Save,
    Close,
}

fn msg(event: Event) -> Message {
    Message::BlockCard(event)
}

/// The words of a tags field: by commas, each trimmed, the empty ones left out.
fn tags_of(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect()
}

impl App {
    /// Kart… on a Bloklar row.
    pub(crate) fn open_block_card(&mut self, id: BlockId) {
        let Some(doc) = &self.document else { return };
        let Some(b) = doc.model.block(id) else { return };
        let card = b.card.clone().unwrap_or_default();
        let mut about = vec![match &b.card {
            Some(c) => format!(
                "Sürüm {}: tanımın çizdiği her değişiklikte artar.",
                c.version
            ),
            None => "Kartı yok: kaydedilince sürümü 1 olur.".to_owned(),
        }];
        if let Some(size) = crate::blocks_panel::size_text(self, id) {
            about.push(size);
        }
        if !b.attributes.is_empty() {
            about.push(format!(
                "Öznitelikleri: {}",
                b.attributes
                    .iter()
                    .map(|a| a.tag.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        self.block_card = Some(Window {
            block: id,
            name: b.name.clone(),
            category: card.category.unwrap_or_default(),
            tags: card.tags.join(", "),
            description: b.description.clone().unwrap_or_default(),
            about,
            refused: None,
        });
        self.dialog = Some(Dialog::BlockCard);
    }

    pub(crate) fn block_card_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.block_card.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Category(t) => w.category = t,
            Event::Tags(t) => w.tags = t,
            Event::Description(t) => w.description = t,
            Event::Close => {
                self.block_card = None;
                self.dialog = None;
            }
            Event::Save => {
                let input = BlocksEdit {
                    dynamic: None,
                    operation: BlockEditOperation::Card,
                    block: Some(w.block),
                    name: None,
                    uids: None,
                    base: None,
                    replace: None,
                    layer_id: None,
                    attributes: None,
                    blocks: None,
                    library: None,
                    entities: None,
                    units: None,
                    rescale: None,
                    sync: None,
                    renames: None,
                    remove_tags: None,
                    tag: None,
                    at: None,
                    category: Some(w.category.trim().to_owned()),
                    tags: Some(tags_of(&w.tags)),
                    description: Some(w.description.trim().to_owned()),
                    expected_revision: None,
                };
                let name = w.name.clone();
                let Some(doc) = self.document.as_mut() else {
                    return Task::none();
                };
                match blocks_edit::execute(&mut ExecutionContext::new(&mut doc.model), input) {
                    CommandResult::Completed { .. } => {
                        self.block_card = None;
                        self.dialog = None;
                        self.say(
                            Level::Success,
                            format!("“{name}” bloğunun kartı kaydedildi."),
                        );
                    }
                    CommandResult::Failed { error }
                    | CommandResult::Conflict { error }
                    | CommandResult::NeedsInput { error } => {
                        if let Some(w) = self.block_card.as_mut() {
                            w.refused = Some(error.message);
                        }
                    }
                    _ => {}
                }
            }
        }
        Task::none()
    }

    pub(crate) fn block_card_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.block_card else {
            return text("").into();
        };
        let mut lines = vec![words::text_line(
            Kind::Info,
            format!(
                "“{}” bloğunun kartı: Bloklar panelinde arama ve kategori süzgeci bunlarla bulur.",
                w.name
            ),
        )];
        lines.extend(
            w.about
                .iter()
                .map(|a| words::text_line(Kind::Info, a.clone())),
        );
        let field = |value: &str, hint: &'static str, on: fn(String) -> Event| {
            focus_ring(
                text_input(hint, value)
                    .on_input(move |t| msg(on(t)))
                    .on_submit(msg(Event::Save))
                    .padding([5, 8])
                    .style(style::field::input),
            )
        };
        let mut body = Column::new()
            .spacing(12)
            .push(words::summary(lines))
            .push(words::field(
                "Kategori",
                field(&w.category, "Mimari/Kapılar", Event::Category),
                Some("Alt kategoriler / ile: Mimari/Kapılar.".to_owned()),
            ))
            .push(words::field(
                "Etiketler",
                field(&w.tags, "kapı, tek kanat", Event::Tags),
                Some("Virgülle ayırın; her etiket bir kez.".to_owned()),
            ))
            .push(words::field(
                "Açıklama",
                field(&w.description, "Tek kanatlı iç kapı", Event::Description),
                None,
            ));
        if let Some(why) = &w.refused {
            body = body.push(words::text_line(Kind::Error, why.clone()));
        }
        overlay::modal(
            Frame::new(CARD_TITLE)
                .push(body)
                .action(words::secondary(CANCEL, Some(msg(Event::Close))))
                .action(words::primary(SAVE, Some(msg(Event::Save))))
                .width(440.0),
            msg(Event::Close),
        )
    }
}
