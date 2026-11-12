//! User-defined property headings, through the ordinary guarded dynamic-definition command (ADR 0858).

use iced::widget::{Column, button, container, row, text};
use iced::{Element, Fill, FillPortion, Task};
use kentos_commands::{BlockId, BlockParameterGroup, BlocksEdit, CommandResult, DynamicBlock};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, blocks_edit};
use kentos_ui::widget::Dialog as Frame;

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};

pub const TITLE: &str = "Parametre grupları";

#[derive(Debug, Clone)]
pub struct Window {
    block: BlockId,
    name: String,
    session: u64,
    revision: String,
    original: DynamicBlock,
    assignments: Vec<String>,
    order: Vec<String>,
    refused: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Assign(usize, String),
    Move(usize, bool),
    Save,
    Close,
}

fn msg(event: Event) -> Message {
    Message::BlockParameterGroups(event)
}

/// Preserve explicit group/member order, append new members in definition order, leave unassigned ones automatic.
fn prepared(w: &Window) -> Result<DynamicBlock, String> {
    use std::collections::{HashMap, HashSet};
    let assigned: HashMap<&str, &str> = w
        .original
        .parameters
        .iter()
        .zip(&w.assignments)
        .map(|(p, name)| (p.name(), name.trim()))
        .collect();
    let mut members: HashMap<&str, Vec<String>> = HashMap::new();
    let mut seen_members = HashSet::new();
    for g in &w.original.parameter_groups {
        for name in &g.parameters {
            if assigned.get(name.as_str()) == Some(&g.name.as_str())
                && seen_members.insert(name.as_str())
            {
                members
                    .entry(g.name.as_str())
                    .or_default()
                    .push(name.clone());
            }
        }
    }
    for p in &w.original.parameters {
        let name = assigned[p.name()];
        if !name.is_empty() && seen_members.insert(p.name()) {
            members.entry(name).or_default().push(p.name().to_owned());
        }
    }
    let mut seen_groups = HashSet::new();
    let mut groups = Vec::new();
    for name in w
        .order
        .iter()
        .map(String::as_str)
        .chain(w.assignments.iter().map(|s| s.trim()))
    {
        if seen_groups.insert(name)
            && let Some(parameters) = members.remove(name)
        {
            groups.push(BlockParameterGroup {
                name: name.to_owned(),
                parameters,
            });
        }
    }
    let mut dynamic = w.original.clone();
    dynamic.parameter_groups = groups;
    // Geometry IDs are not needed for the presentation-only check. The shared command validates the whole block.
    if let Some((_, why)) = kentos_commands::block_dynamic::parameter_groups_problem(&dynamic) {
        return Err(why);
    }
    Ok(dynamic)
}

impl App {
    pub(crate) fn open_block_parameter_groups(&mut self, id: BlockId) {
        let Some(doc) = &self.document else { return };
        let Some(b) = doc.model.block(id) else { return };
        let Some(dynamic) = &b.dynamic else { return };
        self.block_parameter_groups = Some(Window {
            block: id,
            name: b.name.clone(),
            session: doc.session,
            revision: doc.model.revision().to_string(),
            assignments: dynamic
                .parameters
                .iter()
                .map(|p| {
                    dynamic
                        .parameter_groups
                        .iter()
                        .find(|g| g.parameters.iter().any(|n| n == p.name()))
                        .map_or_else(String::new, |g| g.name.clone())
                })
                .collect(),
            order: dynamic
                .parameter_groups
                .iter()
                .map(|g| g.name.clone())
                .collect(),
            original: dynamic.clone(),
            refused: None,
        });
        self.dialog = Some(Dialog::BlockParameterGroups);
    }

    pub(crate) fn block_parameter_groups_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.block_parameter_groups.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Assign(i, value) => {
                if let Some(group) = w.assignments.get_mut(i) {
                    *group = value;
                    w.refused = None;
                }
            }
            Event::Move(i, down) => {
                if let Ok(dynamic) = prepared(w) {
                    let mut order: Vec<String> = dynamic
                        .parameter_groups
                        .into_iter()
                        .map(|g| g.name)
                        .collect();
                    let to = if down {
                        i.checked_add(1)
                    } else {
                        i.checked_sub(1)
                    };
                    if let Some(to) = to.filter(|&to| i < order.len() && to < order.len()) {
                        order.swap(i, to);
                        w.order = order;
                    }
                }
            }
            Event::Close => {
                self.block_parameter_groups = None;
                self.dialog = None;
            }
            Event::Save => {
                let dynamic = match prepared(w) {
                    Ok(d) => d,
                    Err(why) => {
                        w.refused = Some(why);
                        return Task::none();
                    }
                };
                let Some(doc) = self.document.as_mut().filter(|d| d.session == w.session) else {
                    w.refused = Some("Çizim değişti. Pencereyi kapatıp yeniden açın.".into());
                    return Task::none();
                };
                let input: BlocksEdit = match serde_json::from_value(serde_json::json!({
                    "operation": "dynamic", "block": w.block, "dynamic": dynamic,
                    "expectedRevision": w.revision,
                })) {
                    Ok(input) => input,
                    Err(error) => {
                        w.refused = Some(error.to_string());
                        return Task::none();
                    }
                };
                match blocks_edit::execute(&mut ExecutionContext::new(&mut doc.model), input) {
                    CommandResult::Completed { .. } => {
                        self.block_parameter_groups = None;
                        self.dialog = None;
                        self.say(Level::Success, "Parametre grupları kaydedildi.");
                    }
                    CommandResult::Failed { error }
                    | CommandResult::Conflict { error }
                    | CommandResult::NeedsInput { error } => w.refused = Some(error.message),
                    _ => {}
                }
            }
        }
        Task::none()
    }

    pub(crate) fn block_parameter_groups_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.block_parameter_groups else {
            return text("").into();
        };
        let result = prepared(w);
        let mut body = Column::new().spacing(12)
            .push(words::text_line(Kind::Info, format!("“{}” bloğunun özellik başlıkları", w.name)))
            .push(words::text_line(Kind::Info, "Aynı grup adını alan parametreler birlikte gösterilir. Boş alan otomatik başlığı korur. Ölçüler, formüller ve örnek değerleri değişmez."));
        body = body.push(
            row![
                container(kentos_ui::label::caption("Parametre")).width(FillPortion(1)),
                container(kentos_ui::label::caption("Grup adı")).width(FillPortion(1))
            ]
            .spacing(16),
        );
        let mut names = Vec::new();
        for name in w
            .assignments
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            if !names.iter().any(|n| n == name) {
                names.push(name.to_owned());
            }
        }
        let suggestions = std::sync::Arc::new(names);
        for (i, p) in w.original.parameters.iter().enumerate() {
            let names = suggestions.clone();
            let current = w.assignments[i].trim().to_owned();
            let choices = kentos_ui::widget::MenuButton::new(
                container(kentos_ui::icon::icon(kentos_ui::icon::Icon::ChevronDown).size(12.0))
                    .padding(8),
                move || {
                    let mut menu = kentos_ui::widget::Menu::new().radio(
                        "Otomatik grup",
                        current.is_empty(),
                        msg(Event::Assign(i, String::new())),
                    );
                    for name in names.iter() {
                        menu = menu.radio(
                            name.clone(),
                            current == *name,
                            msg(Event::Assign(i, name.clone())),
                        );
                    }
                    menu
                },
            );
            body = body.push(
                row![
                    container(text(p.name()).size(14)).width(FillPortion(1)),
                    container(
                        row![
                            kentos_ui::widget::text_field("Otomatik grup", &w.assignments[i])
                                .on_input(move |value| msg(Event::Assign(i, value))),
                            choices
                        ]
                        .spacing(4)
                        .align_y(iced::Center)
                    )
                    .width(FillPortion(1)),
                ]
                .spacing(16)
                .align_y(iced::Center),
            );
        }
        if let Ok(dynamic) = &result {
            if !dynamic.parameter_groups.is_empty() {
                body = body.push(kentos_ui::label::caption("Paneldeki grup sırası"));
            }
            for (i, g) in dynamic.parameter_groups.iter().enumerate() {
                let up = button(kentos_ui::icon::icon(kentos_ui::icon::Icon::ChevronUp).size(12.0))
                    .style(kentos_ui::style::button::ghost)
                    .on_press_maybe((i > 0).then(|| msg(Event::Move(i, false))));
                let down =
                    button(kentos_ui::icon::icon(kentos_ui::icon::Icon::ChevronDown).size(12.0))
                        .style(kentos_ui::style::button::ghost)
                        .on_press_maybe(
                            (i + 1 < dynamic.parameter_groups.len())
                                .then(|| msg(Event::Move(i, true))),
                        );
                body = body.push(
                    row![
                        container(
                            text(format!("{} · {} parametre", g.name, g.parameters.len())).size(14)
                        )
                        .width(Fill),
                        up,
                        down
                    ]
                    .spacing(4)
                    .align_y(iced::Center),
                );
            }
        }
        if let Some(why) = w.refused.as_ref().or(result.as_ref().err()) {
            body = body.push(words::text_line(Kind::Error, why.clone()));
        }
        Frame::new(TITLE)
            .scroll(body)
            .action(words::secondary("Vazgeç", Some(msg(Event::Close))))
            .action(words::primary(
                "Kaydet",
                result.is_ok().then(|| msg(Event::Save)),
            ))
            .width(640.0)
            .max_height(660.0)
            .modal(msg(Event::Close))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(grouped: bool) -> (App, BlockId) {
        let mut app = crate::files_testing::app_with_drawing();
        let source = if grouped {
            include_bytes!("../../../../fixtures/kcad/v2/block-parameter-groups.kcad").as_slice()
        } else {
            include_bytes!("../../../../fixtures/kcad/v2/block-publication.kcad").as_slice()
        };
        let source = kentos_kcad::decode(source).unwrap();
        let id = source.blocks[0].id;
        app.document = Some(crate::document::Document::from_v2(source, None).unwrap());
        (app, id)
    }

    #[test]
    fn existing_group_and_member_order_survive_a_noop_save_without_revision_or_history() {
        let (mut app, id) = app(true);
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let revision = app.document.as_ref().unwrap().model.revision();
        app.open_block_parameter_groups(id);
        assert_eq!(
            prepared(app.block_parameter_groups.as_ref().unwrap()).unwrap(),
            before.blocks[0].dynamic.clone().unwrap()
        );
        let _ = app.block_parameter_groups_event(Event::Save);
        let doc = &app.document.as_ref().unwrap().model;
        assert_eq!(doc.to_snapshot_v2(), before);
        assert_eq!(doc.revision(), revision);
    }

    #[test]
    fn assignment_and_reordering_keep_geometry_overrides_card_counter_and_exact_undo() {
        let (mut app, id) = app(false);
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        app.open_block_parameter_groups(id);
        for (i, name) in [(0, "Kanat"), (1, "Kanat"), (2, "Görünüş"), (3, "Ürün")] {
            let _ = app.block_parameter_groups_event(Event::Assign(i, name.into()));
        }
        let _ = app.block_parameter_groups_event(Event::Move(2, false));
        let _ = app.block_parameter_groups_event(Event::Save);
        assert!(app.block_parameter_groups.is_none());
        let doc = &mut app.document.as_mut().unwrap().model;
        let after = doc.to_snapshot_v2();
        assert_eq!(after.entities, before.entities);
        assert_eq!(after.blocks[0].entities, before.blocks[0].entities);
        assert_eq!(after.blocks[0].card, before.blocks[0].card);
        let dynamic = after.blocks[0].dynamic.as_ref().unwrap();
        assert_eq!(
            dynamic
                .parameter_groups
                .iter()
                .map(|g| g.name.as_str())
                .collect::<Vec<_>>(),
            ["Kanat", "Ürün", "Görünüş"]
        );
        assert_eq!(
            dynamic.parameters,
            before.blocks[0].dynamic.as_ref().unwrap().parameters
        );
        assert_eq!(
            dynamic.actions,
            before.blocks[0].dynamic.as_ref().unwrap().actions
        );
        let bytes = kentos_kcad::encode_verified(&after).unwrap();
        assert_eq!(
            kentos_kcad::encode(&kentos_kcad::decode(&bytes).unwrap()).unwrap(),
            bytes
        );
        doc.undo().unwrap();
        assert_eq!(doc.to_snapshot_v2(), before);
        doc.redo().unwrap();
        assert_eq!(doc.to_snapshot_v2(), after);
    }

    #[test]
    fn invalid_names_and_cancel_never_change_the_drawing() {
        let (mut app, id) = app(false);
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        app.open_block_parameter_groups(id);
        let _ = app.block_parameter_groups_event(Event::Assign(0, "KANAT".into()));
        let _ = app.block_parameter_groups_event(Event::Assign(1, "kanat".into()));
        assert!(prepared(app.block_parameter_groups.as_ref().unwrap()).is_err());
        let _ = app.block_parameter_groups_event(Event::Save);
        assert!(
            app.block_parameter_groups
                .as_ref()
                .unwrap()
                .refused
                .is_some()
        );
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
        let _ = app.block_parameter_groups_event(Event::Close);
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
    }

    #[test]
    fn stale_revision_and_new_document_session_refuse_to_overwrite() {
        let (mut app, id) = app(false);
        app.open_block_parameter_groups(id);
        let _ = app.block_parameter_groups_event(Event::Assign(0, "Kanat".into()));
        let input: BlocksEdit = serde_json::from_value(
            serde_json::json!({"operation":"card","block":id,"description":"Başka işlem"}),
        )
        .unwrap();
        let doc = &mut app.document.as_mut().unwrap().model;
        assert!(matches!(
            blocks_edit::execute(&mut ExecutionContext::new(doc), input),
            CommandResult::Completed { .. }
        ));
        let after_other_change = doc.to_snapshot_v2();
        let _ = app.block_parameter_groups_event(Event::Save);
        assert!(
            app.block_parameter_groups
                .as_ref()
                .unwrap()
                .refused
                .is_some()
        );
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            after_other_change
        );
        app.document =
            Some(crate::document::Document::from_v2(after_other_change.clone(), None).unwrap());
        let _ = app.block_parameter_groups_event(Event::Save);
        assert!(
            app.block_parameter_groups
                .as_ref()
                .unwrap()
                .refused
                .as_ref()
                .unwrap()
                .contains("Çizim değişti")
        );
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            after_other_change
        );
    }
}
