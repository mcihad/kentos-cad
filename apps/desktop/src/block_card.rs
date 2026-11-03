//! Blok kartı (docs/adr/0317 §4): a block's library card, its category (a path by `/`), the words it is found by
//! and its description, written with `cad.blocks.edit`'s `card` as one undo step “Blok değiştir”. Its version is the
//! drawing's count of the changes to what it draws; the window shows it and its size, unit and attributes.

use iced::widget::{Column, button, row, text};
use iced::{Element, Task};
use kentos_commands::{BlockEditOperation, BlockId, BlockPublication, BlocksEdit, CommandResult};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, blocks_edit};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::widget::Dialog as Frame;

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
    publication: bool,
    family: String,
    type_id: String,
    identity_details: bool,
    type_name: String,
    release: String,
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
    Publication,
    Family(String),
    TypeId(String),
    IdentityDetails,
    TypeName(String),
    Release(String),
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

fn publication_of(w: &Window) -> Result<Option<BlockPublication>, String> {
    if !w.publication {
        return Ok(None);
    }
    Ok(Some(BlockPublication {
        family_id: BlockId::parse(w.family.trim())
            .ok_or("Aile kimliği küçük harfli, tireli bir UUID olmalı.")?,
        type_id: BlockId::parse(w.type_id.trim())
            .ok_or("Tip kimliği küçük harfli, tireli bir UUID olmalı.")?,
        type_name: w.type_name.trim().to_owned(),
        release: w
            .release
            .trim()
            .parse::<u32>()
            .ok()
            .filter(|n| *n > 0)
            .ok_or("Yayın numarası için 1, 2, 3 gibi geçerli bir tam sayı girin.")?,
    }))
}

impl App {
    /// Kart… on a Bloklar row.
    pub(crate) fn open_block_card(&mut self, id: BlockId) {
        let Some(doc) = &self.document else { return };
        let Some(b) = doc.model.block(id) else { return };
        let card = b.card.clone().unwrap_or_default();
        let mut about = vec![match &b.card {
            Some(c) => format!(
                "Yerel değişiklik {}: yayın numarasından bağımsızdır.",
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
            type_id: card.publication.as_ref().map_or_else(
                || kentos_domain::Uuid::new_v4().to_string(),
                |p| p.type_id.to_text(),
            ),
            identity_details: false,
            publication: card.publication.is_some(),
            family: card.publication.as_ref().map_or_else(
                || kentos_domain::Uuid::new_v4().to_string(),
                |p| p.family_id.to_text(),
            ),
            type_name: card
                .publication
                .as_ref()
                .map_or_else(|| b.name.clone(), |p| p.type_name.clone()),
            release: card
                .publication
                .as_ref()
                .map_or(1, |p| p.release)
                .to_string(),
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
            Event::Publication => w.publication = !w.publication,
            Event::Family(t) => w.family = t,
            Event::TypeId(t) => w.type_id = t,
            Event::IdentityDetails => w.identity_details = !w.identity_details,
            Event::TypeName(t) => w.type_name = t,
            Event::Release(t) => w.release = t,
            Event::Close => {
                self.block_card = None;
                self.dialog = None;
            }
            Event::Save => {
                let publication = match publication_of(w) {
                    Ok(p) => p,
                    Err(why) => {
                        if why.contains("kimliği") {
                            w.identity_details = true;
                        }
                        w.refused = Some(why);
                        return Task::none();
                    }
                };
                let input = BlocksEdit {
                    publication,
                    clear_publication: Some(!w.publication),
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
                    rename_nested: None,
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
            kentos_ui::widget::text_field(hint, value)
                .on_input(move |t| msg(on(t)))
                .on_submit(msg(Event::Save))
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
        body = body.push(words::check(
            w.publication,
            "Kitaplık bileşeninin aile, tip ve yayın bilgisi var",
            Some(msg(Event::Publication)),
        ));
        if w.publication {
            body = body
                .push(words::field("Bileşen tipi", field(&w.type_name, "Standart tek kanat", Event::TypeName), None))
                .push(words::field("Yayın numarası", field(&w.release, "1", Event::Release),
                    Some("Kitaplığın yayın numarası; yerel değişiklik sayacı değildir. Kaydetmek çevrimiçi yayın yapmaz.".into())))
                .push(button(row![icon(if w.identity_details { Icon::ChevronDown } else { Icon::ChevronRight }).size(12.0), kentos_ui::label::caption("Kimlik ayrıntıları")].spacing(6).align_y(iced::Center))
                    .on_press(msg(Event::IdentityDetails)).style(kentos_ui::style::button::ghost));
            if w.identity_details {
                body = body
                    .push(words::field("Aile kimliği", field(&w.family, "Ailenin ortak kimliği", Event::Family),
                        Some("Aynı ailenin tiplerinde ve yayınlarında korunur; yeni aile için otomatik üretilir.".into())))
                    .push(words::field("Tip kimliği", field(&w.type_id, "Tipin kalıcı kimliği", Event::TypeId),
                        Some("Tip adı değişse de korunur. Yeni bir tip için farklı kimlik kullanılır.".into())));
            }
        }
        if let Some(why) = &w.refused {
            body = body.push(words::text_line(Kind::Error, why.clone()));
        }
        Frame::new(CARD_TITLE)
            .scroll(body)
            .action(words::secondary(CANCEL, Some(msg(Event::Close))))
            .action(words::primary(SAVE, Some(msg(Event::Save))))
            .width(540.0)
            .max_height(660.0)
            .modal(msg(Event::Close))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publication_fields_validate_cancel_save_and_detach_without_touching_instances() {
        let mut app = crate::files_testing::app_with_drawing();
        let source = kentos_kcad::decode(include_bytes!(
            "../../../fixtures/kcad/v2/block-publication.kcad"
        ))
        .unwrap();
        let id = source.blocks[0].id;
        app.document = Some(crate::document::Document::from_v2(source, None).unwrap());
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        app.open_block_card(id);
        let w = app.block_card.as_ref().unwrap();
        assert!(w.publication);
        assert_eq!(w.type_name, "Standart tek kanat");
        assert_eq!(w.release, "2");
        let _ = app.block_card_event(Event::Release("0".into()));
        let _ = app.block_card_event(Event::Save);
        assert!(app.block_card.as_ref().unwrap().refused.is_some());
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
        let _ = app.block_card_event(Event::Release("3".into()));
        let _ = app.block_card_event(Event::Save);
        assert!(app.block_card.is_none());
        let model = &mut app.document.as_mut().unwrap().model;
        assert_eq!(
            model.blocks()[0]
                .card
                .as_ref()
                .unwrap()
                .publication
                .as_ref()
                .unwrap()
                .release,
            3
        );
        assert_eq!(model.blocks()[0].card.as_ref().unwrap().version, 7);
        assert_eq!(model.to_snapshot_v2().entities, before.entities);
        model.undo().unwrap();
        assert_eq!(model.to_snapshot_v2(), before);
        app.open_block_card(id);
        let _ = app.block_card_event(Event::Publication);
        let _ = app.block_card_event(Event::Close);
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
        app.open_block_card(id);
        let _ = app.block_card_event(Event::Publication);
        let _ = app.block_card_event(Event::Save);
        let model = &mut app.document.as_mut().unwrap().model;
        assert!(
            model.blocks()[0]
                .card
                .as_ref()
                .unwrap()
                .publication
                .is_none()
        );
        assert_eq!(model.to_snapshot_v2().entities, before.entities);
        model.undo().unwrap();
        assert_eq!(model.to_snapshot_v2(), before);
    }
}
