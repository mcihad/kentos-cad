//! Blok düzenleyici (docs/adr/0316 §1, plan/cad.md CAD-BLOCK-05): a block's definition opened as a drawing of its
//! own, every tool at work on it, and its objects written back whole with `cad.blocks.edit`'s `update` as one undo
//! step of the drawing it came from.
//!
//! - **Giriş:** Bloklar panelinin Bloğu düzenle'si, `block.edit` on a selected insert. The drawing steps aside; a
//!   drawing of the definition's objects (its layers, blocks, styles and settings, no other object) takes its
//!   place, each object's number its local id, so a kept object keeps its id and a reference to it follows it
//!   (§1). The block's own layer (`""`, the DXF's 0) is a layer of its own here, “0 (blok)”.
//! - **Çalışma kopyası:** the editor's drawing has its own undo; nothing reaches the drawing until Kaydet.
//!   While it is open, the drawing's file and cloud commands, its cloud work and its recovery copy wait, and the
//!   window does not close.
//! - **Kaydet:** the drawing comes back and its block is updated (Blok değiştir): every insert shows it, a
//!   dimension tied to a piece follows it by its id; a refusal (the block put into itself) keeps the editor open.
//! - **Vazgeç:** the drawing comes back as it was.
//!
//! Both return to the view the drawing was left at.

use iced::widget::{row, text};
use iced::{Element, Task};
use kentos_commands::{
    BlockEditOperation, BlockId, BlocksEdit, CommandResult, Entity, LayerNode, LayerNodeType,
};
use kentos_native_application::{ExecutionContext, blocks_edit};
use kentos_render_wgpu::Camera;
use kentos_ui::{label, style};

use crate::app::{App, Message};
use crate::document::Document;
use kentos_interaction::Level;
use kentos_interaction::Selection;

/// The editor's layer for the block's own layer (`""`).
pub const BLOCK_LAYER: &str = "__blok__";
const BLOCK_LAYER_NAME: &str = "0 (blok)";
/// What the drawing's commands are told while the editor is open.
pub const OPEN: &str = "Blok düzenleyici açık: önce Kaydet ya da Vazgeç ile çizime dönün.";

/// The drawing set aside and the block it edits.
pub struct BlockEditor {
    pub original: Document,
    pub block: BlockId,
    pub name: String,
    camera: Camera,
    selection: Selection,
    test: Option<TestPlacement>,
}

/// The editable draft put aside while an expendable insert is exercised. Its history is kept intact.
struct TestPlacement {
    draft: Document,
    camera: Camera,
    selection: Selection,
}

#[derive(Debug, Clone)]
pub enum Event {
    Open(BlockId),
    Save,
    Discard,
    Test,
    BackToEditor,
}

fn msg(e: Event) -> Message {
    Message::BlockEditor(e)
}

/// The editor's drawing: the drawing's layers (and the block's own layer), blocks, styles and settings, the
/// definition's objects as its objects, each numbered by its local id (an id 0 or met twice the next free one).
fn editor_document(original: &Document, block: BlockId) -> Result<(Document, String), String> {
    let model = &original.model;
    let def = model
        .block(block)
        .ok_or("Blok çizimde tanımlı değil.")?
        .clone();
    let mut snapshot = model.to_snapshot_v2_of(|_| false);
    let known = |id: &str, nodes: &[LayerNode]| -> bool {
        fn find(id: &str, nodes: &[LayerNode]) -> bool {
            nodes
                .iter()
                .any(|n| (n.id == id && n.kind == LayerNodeType::Layer) || find(id, &n.children))
        }
        find(id, nodes)
    };
    let mut used = std::collections::HashSet::new();
    let mut next = def.entities.iter().map(|e| e.base().id).max().unwrap_or(0);
    let mut entities = Vec::with_capacity(def.entities.len());
    for e in &def.entities {
        let mut e = e.clone();
        let base = crate::block_editor::base_mut(&mut e);
        if base.layer_id.is_empty() || !known(&base.layer_id, &snapshot.layers) {
            base.layer_id = BLOCK_LAYER.to_owned();
        }
        if base.id == 0 || !used.insert(base.id) {
            next += 1;
            base.id = next;
            used.insert(next);
        }
        entities.push(e);
    }
    let layer: LayerNode = serde_json::from_value(serde_json::json!({
        "id": BLOCK_LAYER, "name": BLOCK_LAYER_NAME, "type": "layer", "visible": true, "locked": false,
        "expanded": true, "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": []
    }))
    .map_err(|e| e.to_string())?;
    snapshot.layers.insert(0, layer);
    snapshot.active_layer = BLOCK_LAYER.to_owned();
    snapshot.uids = entities
        .iter()
        .map(|_| kentos_commands::EntityId(*kentos_domain::Uuid::now_v7().as_bytes()))
        .collect();
    snapshot.entities = entities;
    snapshot.name = format!("Blok düzenleyici: {}", def.name);
    snapshot.arrays.clear();
    snapshot.sheets = None;
    snapshot.requests.clear();
    let doc = Document::from_v2(snapshot, None)?;
    Ok((doc, def.name))
}

/// An object's base, to renumber and move it.
pub(crate) fn base_mut(e: &mut Entity) -> &mut kentos_commands::EntityBase {
    e.base_mut()
}

/// The editor's objects as the definition holds them: on the block's layer again where they were on its layer,
/// with no drawing order of their own (a definition's objects are drawn in its order).
fn definition_objects(doc: &Document) -> Vec<Entity> {
    doc.model
        .entities()
        .map(|e| {
            let mut e = e.clone();
            let base = e.base_mut();
            if base.layer_id == BLOCK_LAYER {
                base.layer_id = String::new();
            }
            base.look.order = None;
            base.label_pins.clear();
            base.attachments.clear();
            e
        })
        .collect()
}

fn update_input(doc: &Document, block: BlockId) -> BlocksEdit {
    BlocksEdit {
        entities: Some(definition_objects(doc)),
        ..kentos_interaction::blocks::input(BlockEditOperation::Update, Some(block))
    }
}

/// Current, unsaved geometry becomes a definition in a separate document. Normal insertion applies the existing
/// block-unit, attribute and dynamic rules; nothing is recorded in the original or the editable draft.
fn test_document(draft: &Document, editor: &BlockEditor) -> Result<Document, String> {
    let input = update_input(draft, editor.block);
    let mut probe = editor.original.model.clone();
    match blocks_edit::validate(&ExecutionContext::new(&mut probe), &input) {
        CommandResult::Completed { .. } => {}
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => return Err(error.message),
        _ => return Err("Blok doğrulanamadı.".into()),
    }
    let mut snapshot = draft.model.to_snapshot_v2_of(|_| false);
    let def = snapshot
        .blocks
        .iter_mut()
        .find(|b| b.id == editor.block)
        .ok_or("Blok tanımı bulunamadı.")?;
    def.entities = input.entities.unwrap_or_default();
    snapshot.name = format!("Blok testi: {}", editor.name);
    let mut doc = Document::from_v2(snapshot, None)?;
    let result = kentos_native_application::registry::run(
        &mut ExecutionContext::new(&mut doc.model),
        "cad.blocks.insert",
        kentos_commands::CallOp::Execute,
        serde_json::json!({"block":editor.block,"at":{"x":0.,"y":0.},"layerId":BLOCK_LAYER}),
    )
    .map_err(|e| format!("Test yerleştirmesi yapılamadı: {e:?}"))?;
    if result["status"] != "completed" {
        return Err(result["error"]["message"]
            .as_str()
            .unwrap_or("Test yerleştirmesi yapılamadı.")
            .to_owned());
    }
    // Inserting the sample is setup, not the first user undo step. Reopen in memory for a fresh history.
    Document::from_v2(doc.model.to_snapshot_v2(), None)
}

impl App {
    pub(crate) fn block_editor_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Open(block) => self.block_editor_open(block),
            Event::Save => self.block_editor_save(),
            Event::Discard if self.block_editor.as_ref().is_some_and(|e| e.test.is_some()) => {
                self.block_editor_back_from_test()
            }
            Event::Discard => self.block_editor_close(false),
            Event::Test => self.block_editor_test(),
            Event::BackToEditor => self.block_editor_back_from_test(),
        }
        Task::none()
    }

    /// `block.edit`: the selected insert's block.
    pub(crate) fn block_editor_from_selection(&mut self) {
        let block = self.document.as_ref().and_then(|d| {
            let mut ids = self.selection.ids().iter();
            let (Some(slot), None) = (ids.next(), ids.next()) else {
                return None;
            };
            match d.model.get(*slot) {
                Some(Entity::Insert(i)) => Some(i.block),
                _ => None,
            }
        });
        match block {
            Some(b) => self.block_editor_open(b),
            None => self.warn(
                "Düzenlenecek bloğun bir yerleştirmesini seçin ya da Bloklar panelinde bloğun menüsünden Bloğu düzenle'yi seçin.",
            ),
        }
    }

    fn block_editor_open(&mut self, block: BlockId) {
        if self.block_editor.is_some() {
            self.warn(OPEN);
            return;
        }
        // An external reference's content is its source's (docs/adr/0351 §5): opened from Dış başvurular › Kaynağı aç.
        if let Some(name) = self
            .document
            .as_ref()
            .and_then(|d| d.model.block(block))
            .filter(|b| b.xref.is_some())
            .map(|b| b.name.clone())
        {
            self.warn(format!(
                "“{name}” bir dış başvuru: içeriği kaynak dosyasındadır, burada düzenlenmez. Dış başvurular › Kaynağı aç ile kaynağı açın."
            ));
            return;
        }
        let Some(original) = self.document.take() else {
            return;
        };
        match editor_document(&original, block) {
            Ok((doc, name)) => {
                self.cancel();
                let camera = self.viewport.camera;
                let selection = std::mem::replace(&mut self.selection, Selection::new());
                self.spatial.reload(&doc.model);
                self.viewport.opened(&doc, self.spatial.extent());
                self.followed = Some((doc.session, doc.model.generation()));
                let count = doc.entity_count();
                self.document = Some(doc);
                self.block_editor = Some(BlockEditor {
                    original,
                    block,
                    name: name.clone(),
                    camera,
                    selection,
                    test: None,
                });
                self.say(
                    Level::Info,
                    format!(
                        "Blok düzenleyici: “{name}” ({count} nesne). Bütün araçlar tanımın nesneleriyle çalışır; Kaydet çizime yazar, Vazgeç dokunmadan döner."
                    ),
                );
            }
            Err(why) => {
                self.document = Some(original);
                self.warn(format!("Blok düzenleyici açılamadı: {why}"));
            }
        }
    }

    fn block_editor_test(&mut self) {
        let (Some(draft), Some(editor)) = (self.document.as_ref(), self.block_editor.as_ref())
        else {
            return;
        };
        if editor.test.is_some() {
            return;
        }
        let doc = match test_document(draft, editor) {
            Ok(doc) => doc,
            Err(why) => {
                self.warn(format!("Blok denenemedi: {why}"));
                return;
            }
        };
        let selection = std::mem::replace(&mut self.selection, Selection::new());
        self.cancel();
        self.session.exit();
        let camera = self.viewport.camera;
        self.spatial.reload(&doc.model);
        self.viewport.opened(&doc, self.spatial.extent());
        self.followed = Some((doc.session, doc.model.generation()));
        self.selection.set(
            doc.model
                .entities()
                .map(|e| kentos_domain::Slot(e.base().id)),
        );
        let draft = self.document.replace(doc).expect("the editor draft");
        self.block_editor.as_mut().expect("the editor").test = Some(TestPlacement {
            draft,
            camera,
            selection,
        });
        self.say(Level::Info, "Blok testi: boyut, görünürlük ve tutamaçları deneyin. Denemedeki değişiklikler çizime ya da tanıma kaydedilmez; Düzenleyiciye dön taslağınızı geri getirir.");
    }

    fn block_editor_back_from_test(&mut self) {
        let Some(test) = self.block_editor.as_mut().and_then(|e| e.test.take()) else {
            return;
        };
        self.cancel();
        self.session.exit();
        self.spatial.reload(&test.draft.model);
        self.followed = Some((test.draft.session, test.draft.model.generation()));
        self.document = Some(test.draft);
        self.viewport.camera = test.camera;
        if self.viewport.bounds.width > 0.0 && self.viewport.bounds.height > 0.0 {
            self.viewport.camera.set_size(
                f64::from(self.viewport.bounds.width),
                f64::from(self.viewport.bounds.height),
            );
        }
        self.selection = test.selection;
        self.say(Level::Info, "Blok düzenleyiciye dönüldü; test değişiklikleri bırakıldı, kaydedilmemiş taslak korundu.");
    }

    /// Kaydet: the drawing back, its block updated as one step; a refusal keeps the editor open.
    fn block_editor_save(&mut self) {
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        let Some(editor) = self.block_editor.as_ref() else {
            return;
        };
        if editor.test.is_some() {
            self.warn("Test yerleştirmesi kaydedilmez. Önce Düzenleyiciye dön ile taslağa dönün.");
            return;
        }
        let input = update_input(doc, editor.block);
        // Checked against the drawing first: a refusal leaves the editor as it is.
        let mut probe = editor.original.model.clone();
        let plan = blocks_edit::validate(&ExecutionContext::new(&mut probe), &input);
        if let CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } = plan
        {
            self.warn(format!("Blok kaydedilmedi: {}", error.message));
            return;
        }
        let name = editor.name.clone();
        self.block_editor_close(true);
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        match blocks_edit::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, .. } => {
                let inserts = doc
                    .model
                    .entities()
                    .filter(|e| matches!(e, Entity::Insert(i) if output.changed.contains(&i.block)))
                    .count();
                self.say(
                    Level::Success,
                    if output.changed.is_empty() {
                        format!("“{name}” bloğunda değişiklik yoktu.")
                    } else {
                        format!(
                            "“{name}” bloğu kaydedildi; {inserts} yerleştirmesi yeni hâlini gösteriyor. Tek adımda geri alınabilir."
                        )
                    },
                );
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.warn(error.message),
            _ => {}
        }
    }

    /// The drawing back where it was left (Vazgeç; Kaydet writes after).
    fn block_editor_close(&mut self, saving: bool) {
        let Some(editor) = self.block_editor.take() else {
            return;
        };
        self.cancel();
        let BlockEditor {
            original,
            camera,
            selection,
            name,
            ..
        } = editor;
        self.spatial.reload(&original.model);
        self.followed = Some((original.session, original.model.generation()));
        self.document = Some(original);
        self.viewport.camera = camera;
        if self.viewport.bounds.width > 0.0 && self.viewport.bounds.height > 0.0 {
            self.viewport.camera.set_size(
                f64::from(self.viewport.bounds.width),
                f64::from(self.viewport.bounds.height),
            );
        }
        self.selection = selection;
        if !saving {
            self.say(
                Level::Info,
                format!("Blok düzenleyici kapandı; “{name}” değişmedi."),
            );
        }
    }

    /// The bar under the drawing while the editor is open: the block, its objects, Kaydet and Vazgeç.
    pub(crate) fn block_editor_bar(&self) -> Option<Element<'_, Message>> {
        let editor = self.block_editor.as_ref()?;
        if editor.test.is_some() {
            return Some(
                row![
                    label::body(format!("Blok testi: {}", editor.name)),
                    text("Deneme değişiklikleri kaydedilmez").style(style::text::muted),
                    iced::widget::Space::new().width(iced::Fill),
                    crate::exchange::words::primary(
                        "Düzenleyiciye dön",
                        Some(msg(Event::BackToEditor))
                    ),
                ]
                .spacing(12)
                .padding([4, 12])
                .align_y(iced::Center)
                .into(),
            );
        }
        let count = self.document.as_ref().map_or(0, |d| d.entity_count());
        Some(
            row![
                label::body(format!("Blok düzenleyici: {}", editor.name)),
                text(format!("{count} nesne")).style(style::text::muted),
                iced::widget::Space::new().width(iced::Fill),
                crate::exchange::words::secondary("Test et", Some(msg(Event::Test))),
                crate::exchange::words::secondary("Vazgeç", Some(msg(Event::Discard))),
                crate::exchange::words::primary("Kaydet", Some(msg(Event::Save))),
            ]
            .spacing(12)
            .padding([4, 12])
            .align_y(iced::Center)
            .into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::app::{App, Message};
    use crate::block_scenes::{door, editing, handle};

    use super::Event;

    fn pieces(app: &App) -> usize {
        app.document
            .as_ref()
            .and_then(|d| d.model.block(door()).map(|b| b.entities.len()))
            .expect("the door")
    }

    #[test]
    fn a_block_is_edited_saved_as_one_step_and_a_discard_leaves_it() {
        let (mut app, _) = App::boot(None);
        editing(&mut app);
        let editor = app.document.as_ref().expect("the editor's drawing");
        // The definition's four objects are the editor's drawing; the doors and the table are not.
        assert_eq!(editor.entity_count(), 4);
        assert!(app.block_editor.is_some());
        // The drawing's file commands wait while it is open.
        let _ = app.update(Message::Run("file.save"));
        assert!(app.block_editor.is_some());
        handle(&mut app);
        let _ = app.update(Message::BlockEditor(Event::Save));
        assert!(app.block_editor.is_none());
        // The block has the handle; the drawing is back with its own objects and its store sees the new piece.
        assert_eq!(pieces(&app), 5);
        let doc = app.document.as_ref().expect("the drawing");
        assert!(doc.entity_count() > 4);
        assert!(doc.model.can_undo());
        let before = app.spatial.extent();
        assert!(before.is_some());
        // One undo step takes it back whole.
        let _ = app.update(Message::Run("edit.undo"));
        assert_eq!(pieces(&app), 4);
        // Vazgeç: nothing changes.
        editing(&mut app);
        handle(&mut app);
        let _ = app.update(Message::BlockEditor(Event::Discard));
        assert!(app.block_editor.is_none());
        assert_eq!(pieces(&app), 4);
    }

    #[test]
    fn test_placement_uses_unsaved_geometry_and_restores_draft_history_exactly() {
        let (mut app, _) = App::boot(None);
        editing(&mut app);
        let original = app
            .block_editor
            .as_ref()
            .unwrap()
            .original
            .model
            .to_snapshot_v2();
        handle(&mut app);
        let draft_slot = kentos_domain::Slot(
            app.document
                .as_ref()
                .unwrap()
                .model
                .entities()
                .next()
                .unwrap()
                .base()
                .id,
        );
        app.selection.set([draft_slot]);
        let draft = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let draft_history: Vec<_> = app
            .document
            .as_ref()
            .unwrap()
            .model
            .journal()
            .cloned()
            .collect();
        let camera = app.viewport.camera;
        let _ = app.update(Message::BlockEditor(Event::Test));
        assert!(app.block_editor.as_ref().unwrap().test.is_some());
        assert_eq!(app.document.as_ref().unwrap().entity_count(), 1);
        assert!(!app.document.as_ref().unwrap().model.can_undo());
        assert_eq!(
            pieces(&app),
            5,
            "the unsaved handle is in the tested definition"
        );
        assert_eq!(
            app.block_editor
                .as_ref()
                .unwrap()
                .original
                .model
                .to_snapshot_v2(),
            original
        );
        handle(&mut app); // a change in the expendable document only
        assert_eq!(app.document.as_ref().unwrap().entity_count(), 2);
        let _ = app.update(Message::Run("edit.undo"));
        assert_eq!(app.document.as_ref().unwrap().entity_count(), 1);
        assert!(
            !app.document.as_ref().unwrap().model.can_undo(),
            "setup cannot be undone"
        );
        let _ = app.update(Message::BlockEditor(Event::Save));
        assert!(
            app.block_editor.as_ref().unwrap().test.is_some(),
            "test is never saved into the original"
        );
        let _ = app.update(Message::Run("file.save"));
        assert!(app.saving.is_none());
        // The window can be resized during a test. Returning preserves the view, not stale canvas dimensions.
        let bounds = iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(640., 360.));
        app.viewport.update(
            crate::viewport::Event::Resized(bounds),
            app.document.as_ref(),
        );
        let mut expected_camera = camera;
        expected_camera.set_size(640., 360.);
        let _ = app.update(Message::BlockEditor(Event::BackToEditor));
        assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(), draft);
        assert_eq!(
            app.document
                .as_ref()
                .unwrap()
                .model
                .journal()
                .cloned()
                .collect::<Vec<_>>(),
            draft_history
        );
        assert_eq!(app.viewport.camera, expected_camera);
        assert_eq!(app.selection.ids(), &[draft_slot]);
        // Saving after return applies just the draft, in the original drawing's single Undo step.
        let _ = app.update(Message::BlockEditor(Event::Save));
        assert!(app.block_editor.is_none());
        assert_eq!(pieces(&app), 5);
        let _ = app.update(Message::Run("edit.undo"));
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            original
        );
    }

    #[test]
    fn dynamic_test_changes_parameters_and_visibility_without_changing_definition_or_draft() {
        use crate::properties::{Event as Property, Field};
        use kentos_commands::{DrawingUnit, ParamValue};
        let mut snapshot = kentos_kcad::decode(include_bytes!(
            "../../../fixtures/kcad/v2/dynamic-blocks.kcad"
        ))
        .unwrap();
        snapshot.settings.drawing_unit = Some(DrawingUnit::Mm);
        let id = snapshot.blocks[0].id;
        let doc = crate::document::Document::from_v2(snapshot, None).unwrap();
        let (mut app, _) = App::boot(None);
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        let original = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let _ = app.update(Message::BlockEditor(Event::Open(id)));
        let draft = app.document.as_ref().unwrap().model.to_snapshot_v2();
        for _ in 0..2 {
            let _ = app.update(Message::BlockEditor(Event::Test));
            let model = &app.document.as_ref().unwrap().model;
            let slot = kentos_domain::Slot(model.entities().next().unwrap().base().id);
            let kentos_commands::Entity::Insert(i) = model.get(slot).unwrap() else {
                panic!()
            };
            assert!(i.parameters.is_empty(), "each test starts at the defaults");
            let _ = app.update(Message::Properties(Property::Commit(
                Field::Parameter(slot, "Genişlik".into()),
                "1050".into(),
            )));
            let _ = app.update(Message::Properties(Property::Parameter(
                slot,
                "Görünüş".into(),
                ParamValue::Text("Sade".into()),
            )));
            let model = &app.document.as_ref().unwrap().model;
            let kentos_commands::Entity::Insert(i) = model.get(slot).unwrap() else {
                panic!()
            };
            assert_eq!(i.parameters["Genişlik"], ParamValue::Number(1.05));
            assert_eq!(i.parameters["Görünüş"], ParamValue::Text("Sade".into()));
            assert_eq!(
                app.block_editor
                    .as_ref()
                    .unwrap()
                    .original
                    .model
                    .to_snapshot_v2(),
                original
            );
            let _ = app.update(Message::BlockEditor(Event::BackToEditor));
            assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(), draft);
        }
        let _ = app.update(Message::BlockEditor(Event::Discard));
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            original
        );
    }

    #[test]
    fn invalid_self_reference_refuses_test_without_leaving_or_altering_the_draft() {
        let (mut app, _) = App::boot(None);
        editing(&mut app);
        let model = &mut app.document.as_mut().unwrap().model;
        let result = kentos_native_application::registry::run(
            &mut kentos_native_application::ExecutionContext::new(model),
            "cad.blocks.insert",
            kentos_commands::CallOp::Execute,
            serde_json::json!({"block":door(),"at":{"x":0.,"y":0.},"layerId":super::BLOCK_LAYER}),
        )
        .unwrap();
        assert_eq!(result["status"], "completed");
        let draft = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let _ = app.update(Message::BlockEditor(Event::Test));
        assert!(app.block_editor.as_ref().unwrap().test.is_none());
        assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(), draft);
    }
}
