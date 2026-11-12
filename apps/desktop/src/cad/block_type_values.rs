//! Shared drawing-type values and explicit scope migration, using cad.blocks.edit (ADR 0863).
use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::properties::rows::params::{bounds, shown, typed};
use iced::widget::{Column, container, row, text};
use iced::{Element, Fill, FillPortion, Task};
use kentos_commands::{
    BlockId, BlockParameter, BlocksEdit, CommandResult, DynamicBlock, Entity, ParamValue,
};
use kentos_interaction::{Format, Level};
use kentos_native_application::{ExecutionContext, blocks_edit};
use kentos_ui::widget::{Dialog as Frame, Select, select::Choice};
use std::collections::BTreeMap;

pub const TITLE: &str = "Tip değerleri";
#[derive(Debug, Clone)]
pub struct Window {
    block: BlockId,
    name: String,
    session: u64,
    revision: String,
    original: DynamicBlock,
    scoped: Vec<bool>,
    values: Vec<String>,
    format: Format,
    factor: f64,
    direct: usize,
    nested: usize,
    refused: Option<String>,
}
#[derive(Debug, Clone)]
pub enum Event {
    Scope(usize, bool),
    Value(usize, String),
    Lookup(usize, usize),
    Save,
    Close,
}
fn msg(e: Event) -> Message {
    Message::BlockTypeValues(e)
}
// Scope depends on the selected targets, even while one typed value is invalid.
fn lookup_scope(w: &Window, rows: &[kentos_commands::LookupRow]) -> Option<bool> {
    let mut scopes = rows
        .iter()
        .flat_map(|r| r.values.keys())
        .filter_map(|name| {
            w.original
                .parameters
                .iter()
                .position(|p| p.name() == name)
                .map(|i| w.scoped[i])
        });
    let first = scopes.next()?;
    scopes.all(|v| v == first).then_some(first)
}
fn prepared(w: &Window) -> Result<DynamicBlock, String> {
    let mut d = w.original.clone();
    d.type_parameters.clear();
    d.type_values.clear();
    for (i, p) in d.parameters.iter().enumerate() {
        if !w.scoped[i] || matches!(p, BlockParameter::Lookup { .. }) {
            continue;
        }
        let previous = w
            .original
            .type_values
            .get(p.name())
            .cloned()
            .or_else(|| p.default_value());
        let unchanged = previous
            .as_ref()
            .filter(|v| shown(p, v, w.format, w.factor) == w.values[i]);
        let value = if let Some(value) = unchanged {
            Some(value.clone())
        } else {
            match p {
                BlockParameter::Flip { .. } => match w.values[i].as_str() {
                    "Evet" => Some(ParamValue::Flag(true)),
                    "Hayır" => Some(ParamValue::Flag(false)),
                    _ => None,
                },
                BlockParameter::Visibility { .. } => Some(ParamValue::Text(w.values[i].clone())),
                _ => typed(p, &w.values[i], w.format, w.factor),
            }
        }
        .ok_or_else(|| format!("“{}” için geçerli bir değer girin.", p.name()))?;
        if let Some(why) = kentos_commands::block_dynamic::value_problem(p, &value) {
            let limits = bounds(p, w.format, w.factor);
            let detail = if limits.is_empty() {
                why
            } else {
                format!(
                    "İzin verilen aralığa, adıma veya listeye uyan bir değer girin. {}",
                    limits.join(" · ")
                )
            };
            return Err(format!("“{}”: {detail}", p.name()));
        }
        d.type_parameters.push(p.name().to_owned());
        d.type_values.insert(p.name().to_owned(), value);
    }
    let selected = d.type_parameters.clone();
    d.type_parameters = w
        .original
        .type_parameters
        .iter()
        .filter(|n| selected.contains(n))
        .cloned()
        .chain(
            selected
                .iter()
                .filter(|n| !w.original.type_parameters.contains(n))
                .cloned(),
        )
        .collect();
    if let Some((_, why)) = kentos_commands::block_dynamic::type_parameters_problem(&d) {
        return Err(why);
    }
    Ok(d)
}
impl App {
    pub(crate) fn open_block_type_values(&mut self, id: BlockId) {
        let Some(doc) = &self.document else { return };
        let Some(b) = doc.model.block(id) else { return };
        let Some(d) = &b.dynamic else { return };
        let format = Format::of(doc.model.settings());
        let factor = kentos_native_application::blocks_insert::unit_factor(&doc.model, b);
        let current = kentos_native_application::block_params::current(d, &BTreeMap::new());
        self.block_type_values = Some(Window {
            block: id,
            name: b.name.clone(),
            session: doc.session,
            revision: doc.model.revision().to_string(),
            original: d.clone(),
            scoped: d
                .parameters
                .iter()
                .map(|p| d.is_type_parameter(p.name()))
                .collect(),
            values: d
                .parameters
                .iter()
                .map(|p| {
                    current
                        .get(p.name())
                        .map(|v| shown(p, v, format, factor))
                        .unwrap_or_default()
                })
                .collect(),
            format,
            factor,
            direct: doc
                .model
                .entities()
                .filter(|e| matches!(e,Entity::Insert(i) if i.block==id))
                .count(),
            nested: doc
                .model
                .blocks()
                .iter()
                .flat_map(|b| &b.entities)
                .filter(|e| matches!(e,Entity::Insert(i) if i.block==id))
                .count(),
            refused: None,
        });
        self.dialog = Some(Dialog::BlockTypeValues);
    }
    pub(crate) fn block_type_values_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.block_type_values.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Scope(i, value) => {
                if let Some(v) = w.scoped.get_mut(i) {
                    *v = value;
                    w.refused = None;
                }
            }
            Event::Value(i, value) => {
                if let Some(v) = w.values.get_mut(i) {
                    *v = value;
                    w.refused = None;
                }
            }
            Event::Lookup(index, row) => {
                if let Some(BlockParameter::Lookup { rows, .. }) = w.original.parameters.get(index)
                    && lookup_scope(w, rows) == Some(true)
                    && let Some(row) = rows.get(row)
                {
                    for (name, value) in &row.values {
                        if let Some((i, p)) = w
                            .original
                            .parameters
                            .iter()
                            .enumerate()
                            .find(|(_, p)| p.name() == name)
                        {
                            w.values[i] = shown(p, value, w.format, w.factor);
                        }
                    }
                    w.refused = None;
                }
            }
            Event::Close => {
                self.block_type_values = None;
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
                let input: BlocksEdit = match serde_json::from_value(
                    serde_json::json!({"operation":"dynamic","block":w.block,"dynamic":dynamic,"expectedRevision":w.revision}),
                ) {
                    Ok(v) => v,
                    Err(e) => {
                        w.refused = Some(e.to_string());
                        return Task::none();
                    }
                };
                match blocks_edit::execute(&mut ExecutionContext::new(&mut doc.model), input) {
                    CommandResult::Completed { .. } => {
                        self.block_type_values = None;
                        self.dialog = None;
                        self.say(Level::Success, "Tip değerleri kaydedildi.");
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
    pub(crate) fn block_type_values_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.block_type_values else {
            return text("").into();
        };
        let result = prepared(w);
        let mut body=Column::new().spacing(14)
            .push(words::text_line(Kind::Info,format!("“{}” · {} doğrudan, {} tanım içi kullanım",w.name,w.direct,w.nested)))
            .push(words::text_line(Kind::Info,"Tip değeri bu çizimde aynı tanımın bütün kullanımlarını değiştirir. Örnek değeri yalnız seçilen yerleştirmede düzenlenir. Ayrı tanımlar ve eski yayınlar birbirinden bağımsızdır."))
            .push(words::text_line(Kind::Info,"Kapsamı tipe geçirirken farklı örnek değerleri varsa işlem reddedilir. Örneğe dönünce mevcut ortak değer her kullanıma aktarılır. Kaydet tek geri alma adımıdır."));
        for (i, p) in w.original.parameters.iter().enumerate() {
            let derived = p.rule().is_some_and(|r| r.expression.is_some());
            let scope: Element<'_, Message> = if let BlockParameter::Lookup { rows, .. } = p {
                text(match lookup_scope(w, rows) {
                    Some(true) => "Tip · hedeflerinden",
                    Some(false) => "Örnek · hedeflerinden",
                    None => "Karma · kapsamları eşitleyin",
                })
                .size(13)
                .into()
            } else {
                Select::new(
                    [Choice::new("Örnek"), Choice::new("Tip · ortak")],
                    Some(usize::from(w.scoped[i])),
                    move |index| msg(Event::Scope(i, index == 1)),
                )
                .into()
            };
            let value: Element<'_, Message> = if derived {
                text(if w.scoped[i] {
                    "Kaydederken hesaplanır"
                } else {
                    "Her örnekte hesaplanır"
                })
                .size(14)
                .into()
            } else if let BlockParameter::Lookup { name, rows } = p {
                if lookup_scope(w, rows) == Some(true) {
                    let values = result
                        .as_ref()
                        .ok()
                        .map(|dynamic| {
                            kentos_native_application::block_params::current(
                                dynamic,
                                &BTreeMap::new(),
                            )
                        })
                        .unwrap_or_default();
                    let label = values.get(name).and_then(|v| match v {
                        ParamValue::Text(t) => Some(t.as_str()),
                        _ => None,
                    });
                    Select::new(
                        rows.iter().map(|r| Choice::new(r.label.clone())),
                        rows.iter().position(|r| Some(r.label.as_str()) == label),
                        move |r| msg(Event::Lookup(i, r)),
                    )
                    .placeholder("Özel ortak değer")
                    .into()
                } else {
                    text("Her örnekte seçilir").size(14).into()
                }
            } else if !w.scoped[i] {
                text(format!("Varsayılan: {}", w.values[i])).size(14).into()
            } else {
                match p {
                    BlockParameter::Flip { .. } => Select::new(
                        [Choice::new("Hayır"), Choice::new("Evet")],
                        Some(usize::from(w.values[i] == "Evet")),
                        move |index| {
                            msg(Event::Value(
                                i,
                                if index == 1 { "Evet" } else { "Hayır" }.into(),
                            ))
                        },
                    )
                    .into(),
                    BlockParameter::Visibility { states, .. } => Select::new(
                        states.iter().map(|s| Choice::new(s.name.clone())),
                        states.iter().position(|s| s.name == w.values[i]),
                        move |index| msg(Event::Value(i, states[index].name.clone())),
                    )
                    .into(),
                    _ => kentos_ui::widget::text_field("Ortak değer", &w.values[i])
                        .on_input(move |v| msg(Event::Value(i, v)))
                        .into(),
                }
            };
            let unit = match p {
                BlockParameter::Linear { .. }
                | BlockParameter::Polar { .. }
                | BlockParameter::Point { .. } => w.format.length_unit_label(),
                BlockParameter::Angle { .. } => w.format.angle_unit_label(),
                _ => "",
            };
            body = body.push(
                Column::new().spacing(5).push(text(p.name()).size(14)).push(
                    row![
                        container(scope).width(FillPortion(1)),
                        container(value).width(FillPortion(1)),
                        text(unit).size(12)
                    ]
                    .spacing(12)
                    .align_y(iced::Center),
                ),
            );
            let limits = bounds(p, w.format, w.factor);
            if !limits.is_empty() {
                body = body.push(words::text_line(Kind::Info, limits.join(" · ")));
            }
            if derived {
                body = body.push(words::text_line(
                    Kind::Info,
                    format!(
                        "Hesaplanan · {}",
                        p.rule()
                            .and_then(|r| r.expression.as_deref())
                            .unwrap_or_default()
                    ),
                ));
            }
        }
        if let Some(why) = w.refused.as_ref().or(result.as_ref().err()) {
            body = body.push(words::text_line(Kind::Error, why.clone()));
        }
        Frame::new(TITLE)
            .scroll(container(body).width(Fill))
            .action(words::secondary("Vazgeç", Some(msg(Event::Close))))
            .action(words::primary(
                "Kaydet",
                result.is_ok().then(|| msg(Event::Save)),
            ))
            .width(700.0)
            .max_height(720.0)
            .modal(msg(Event::Close))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn app() -> (App, BlockId) {
        let mut app = crate::files_testing::app_with_drawing();
        let snapshot = kentos_kcad::decode(include_bytes!(
            "../../../../fixtures/kcad/v2/block-type-values.kcad"
        ))
        .unwrap();
        let id = snapshot.blocks[0].id;
        app.document = Some(crate::document::Document::from_v2(snapshot, None).unwrap());
        (app, id)
    }
    #[test]
    fn common_value_uses_project_units_and_one_undo_preserves_every_instance_input() {
        let (mut app, id) = app();
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        app.open_block_type_values(id);
        let w = app.block_type_values.as_ref().unwrap();
        assert_eq!(w.values[0], "1.1");
        assert_eq!(w.direct, 2);
        assert_eq!(w.nested, 1);
        let _ = app.block_type_values_event(Event::Value(0, "1.2".into()));
        let _ = app.block_type_values_event(Event::Save);
        assert!(app.block_type_values.is_none());
        let doc = &mut app.document.as_mut().unwrap().model;
        let after = doc.to_snapshot_v2();
        assert_eq!(
            after.blocks[0].dynamic.as_ref().unwrap().type_values["Genişlik"],
            ParamValue::Number(1.2)
        );
        assert_eq!(after.entities, before.entities);
        assert_eq!(after.blocks[1], before.blocks[1]);
        doc.undo();
        assert_eq!(doc.to_snapshot_v2(), before);
    }
    #[test]
    fn stale_dialog_refuses_and_cancel_never_writes_a_common_value() {
        let (mut app, id) = app();
        app.open_block_type_values(id);
        let _ = app.block_type_values_event(Event::Value(0, "1.2".into()));
        let input: BlocksEdit = serde_json::from_value(
            serde_json::json!({"operation":"card","block":id,"description":"Başka işlem"}),
        )
        .unwrap();
        assert!(matches!(
            blocks_edit::execute(
                &mut ExecutionContext::new(&mut app.document.as_mut().unwrap().model),
                input
            ),
            CommandResult::Completed { .. }
        ));
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let _ = app.block_type_values_event(Event::Save);
        assert!(app.block_type_values.as_ref().unwrap().refused.is_some());
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
        let _ = app.block_type_values_event(Event::Close);
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
    }
}

#[cfg(test)]
mod lookup_tests {
    use super::*;
    #[test]
    fn common_catalog_row_changes_its_targets_without_storing_a_lookup_scalar() {
        let mut app = crate::files_testing::app_with_drawing();
        let source = kentos_kcad::decode(include_bytes!(
            "../../../../fixtures/kcad/v2/block-type-values.kcad"
        ))
        .unwrap();
        let id = source.blocks[0].id;
        app.document = Some(crate::document::Document::from_v2(source, None).unwrap());
        app.open_block_type_values(id);
        let _ = app.block_type_values_event(Event::Lookup(3, 0));
        let dynamic = prepared(app.block_type_values.as_ref().unwrap()).unwrap();
        assert_eq!(dynamic.type_values["Genişlik"], ParamValue::Number(0.7));
        assert!(!dynamic.type_values.contains_key("Tip"));
        let _ = app.block_type_values_event(Event::Save);
        assert!(app.block_type_values.is_none());
        let doc = &app.document.as_ref().unwrap().model;
        let d = doc.blocks()[0].dynamic.as_ref().unwrap();
        assert_eq!(d.type_values["Genişlik"], ParamValue::Number(0.7));
    }
}

#[cfg(test)]
mod units_tests {
    use super::*;
    #[test]
    fn millimetre_type_dialog_keeps_canonical_metres_and_noop_float_bits() {
        let mut app = crate::files_testing::app_with_drawing();
        let mut source = kentos_kcad::decode(include_bytes!(
            "../../../../fixtures/kcad/v2/block-type-values.kcad"
        ))
        .unwrap();
        source.settings.drawing_unit = Some(kentos_commands::DrawingUnit::Mm);
        let id = source.blocks[0].id;
        source.blocks[0]
            .dynamic
            .as_mut()
            .unwrap()
            .type_values
            .insert("Genişlik".into(), ParamValue::Number(1.1000000000000003));
        app.document = Some(crate::document::Document::from_v2(source, None).unwrap());
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        app.open_block_type_values(id);
        let d = prepared(app.block_type_values.as_ref().unwrap()).unwrap();
        assert_eq!(d, before.blocks[0].dynamic.clone().unwrap());
        let _ = app.block_type_values_event(Event::Save);
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
        app.open_block_type_values(id);
        let _ = app.block_type_values_event(Event::Value(0, "1200 mm".into()));
        let _ = app.block_type_values_event(Event::Save);
        assert!(app.block_type_values.is_none());
        let doc = &app.document.as_ref().unwrap().model;
        assert_eq!(
            doc.blocks()[0].dynamic.as_ref().unwrap().type_values["Genişlik"],
            ParamValue::Number(1.2)
        );
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;
    #[test]
    fn invalid_mm_value_keeps_common_catalog_scope_and_row_can_repair_it() {
        let (mut app, id) = super::tests::app();
        let mut source = app.document.as_ref().unwrap().model.to_snapshot_v2();
        source.settings.drawing_unit = Some(kentos_commands::DrawingUnit::Mm);
        app.document = Some(crate::document::Document::from_v2(source, None).unwrap());
        app.open_block_type_values(id);
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let _ = app.block_type_values_event(Event::Value(0, "1300".into()));
        let w = app.block_type_values.as_ref().unwrap();
        let why = prepared(w).unwrap_err();
        assert!(
            why.contains("Genişlik") && why.contains("1200 mm") && why.contains("50 mm"),
            "{why}"
        );
        let BlockParameter::Lookup { rows, .. } = &w.original.parameters[3] else {
            panic!("lookup")
        };
        assert_eq!(lookup_scope(w, rows), Some(true));
        let _ = app.block_type_values_event(Event::Save);
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
        let _ = app.block_type_values_event(Event::Lookup(3, 0));
        assert_eq!(
            prepared(app.block_type_values.as_ref().unwrap())
                .unwrap()
                .type_values["Genişlik"],
            ParamValue::Number(0.7)
        );
    }
}
