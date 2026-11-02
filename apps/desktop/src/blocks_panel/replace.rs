//! Explicit placement scope and a guarded read-only preview before BLOCKREPLACE.
use crate::{
    app::{App, Dialog, Message},
    exchange::words,
};
use iced::{
    Element, Task,
    widget::{Column, text},
};
use kentos_commands::{
    BlockId, BlocksReplace, CallAnswer, CallEnvelope, CallExpectations, CallOp, CommandResult,
    Entity,
};
use kentos_ui::{label, style, widget::Dialog as Frame};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

pub(crate) const TITLE: &str = "Blok yerleştirmelerini değiştir";
static NEXT: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub(super) struct Window {
    from: BlockId,
    to: BlockId,
    selected: Vec<String>,
    all: Vec<String>,
    only_selected: bool,
    names: (String, String),
    session: u64,
    generation: u64,
    token: u64,
    cancel: Arc<AtomicBool>,
    answer: Option<CallAnswer>,
    refused: Option<String>,
    nested: usize,
}
impl Drop for Window {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
#[derive(Debug, Clone)]
pub enum Event {
    Scope(bool),
    Ready(u64, Option<Box<CallAnswer>>),
    Apply,
    Close,
}
fn msg(e: Event) -> Message {
    super::msg(super::Event::Replace(e))
}
fn envelope(input: &BlocksReplace, op: CallOp, plan: Option<String>) -> CallEnvelope {
    CallEnvelope {
        command: kentos_commands::CAD_BLOCKS_REPLACE.into(),
        version: None,
        op,
        input: serde_json::to_value(input).expect("block replacement serializes"),
        request_id: None,
        expected: plan.map(|plan| CallExpectations {
            plan: Some(plan),
            ..Default::default()
        }),
        relax_constraints: false,
    }
}
fn refusal(answer: &CallAnswer) -> Option<&str> {
    match &answer.result {
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => Some(&error.message),
        _ => None,
    }
}
impl Window {
    fn input(&self) -> BlocksReplace {
        BlocksReplace {
            from: Some(self.from),
            to: Some(self.to),
            uids: Some(if self.only_selected {
                self.selected.clone()
            } else {
                self.all.clone()
            }),
            purge: None,
            expected_revision: None,
        }
    }
}
impl App {
    pub(super) fn blocks_replace(&mut self, from: BlockId, to: BlockId) -> Task<Message> {
        let Some(doc) = self.document.as_ref() else {
            return Task::none();
        };
        let (Some(a), Some(b)) = (doc.model.block(from), doc.model.block(to)) else {
            return Task::none();
        };
        let selected = self
            .selection
            .ids()
            .iter()
            .filter(|s| matches!(doc.model.get(**s),Some(Entity::Insert(i)) if i.block==from))
            .filter_map(|s| doc.model.uid(*s).map(|u| u.to_string()))
            .collect::<Vec<_>>();
        let all = doc
            .model
            .entities_with_uids()
            .filter(|(_, e)| matches!(e,Entity::Insert(i) if i.block==from))
            .map(|(u, _)| u.to_string())
            .collect();
        let name = |b: &kentos_commands::BlockDefinition| {
            b.card.as_ref().map_or_else(
                || format!("{} · kartı yok", b.name),
                |c| format!("{} · sürüm {}", b.name, c.version),
            )
        };
        self.blocks_panel.replacement = Some(Window {
            from,
            to,
            only_selected: !selected.is_empty(),
            selected,
            all,
            names: (name(a), name(b)),
            session: doc.session,
            generation: doc.model.generation(),
            token: 0,
            cancel: Arc::new(AtomicBool::new(false)),
            answer: None,
            refused: None,
            nested: doc
                .model
                .blocks()
                .iter()
                .flat_map(|b| b.entities.iter())
                .filter(|e| matches!(e,Entity::Insert(i) if i.block==from))
                .count(),
        });
        self.dialog = Some(Dialog::BlockReplace);
        self.block_replace_plan()
    }
    fn block_replace_plan(&mut self) -> Task<Message> {
        let (Some(doc), Some(w)) = (
            self.document.as_ref(),
            self.blocks_panel.replacement.as_mut(),
        ) else {
            return Task::none();
        };
        if doc.session != w.session || doc.model.generation() != w.generation {
            w.refused = Some("Çizim değişti. Pencereyi kapatıp kapsamı yeniden inceleyin.".into());
            w.answer = None;
            return Task::none();
        }
        w.cancel.store(true, Ordering::Relaxed);
        w.cancel = Arc::new(AtomicBool::new(false));
        w.token = NEXT.fetch_add(1, Ordering::Relaxed);
        w.answer = None;
        w.refused = None;
        let token = w.token;
        let cancel = Arc::clone(&w.cancel);
        let input = w.input();
        let mut model = doc.model.clone();
        crate::db_connections::off_thread(
            move || {
                if cancel.load(Ordering::Relaxed) {
                    return None;
                }
                let a = kentos_native_application::call::call(
                    &mut model,
                    None,
                    envelope(&input, CallOp::Plan, None),
                );
                if cancel.load(Ordering::Relaxed) {
                    None
                } else {
                    Some(a)
                }
            },
            move |answer| msg(Event::Ready(token, answer.map(Box::new))),
        )
    }
    pub(crate) fn block_replace_close(&mut self) {
        self.blocks_panel.replacement = None;
        self.dialog = None;
    }
    pub(super) fn block_replace_event(&mut self, e: Event) -> Task<Message> {
        if self.dialog != Some(Dialog::BlockReplace) {
            return Task::none();
        }
        match e {
            Event::Close => self.block_replace_close(),
            Event::Scope(selected) => {
                if let Some(w) = self.blocks_panel.replacement.as_mut() {
                    w.only_selected = selected;
                }
                return self.block_replace_plan();
            }
            Event::Ready(token, answer) => {
                if let Some(w) = self
                    .blocks_panel
                    .replacement
                    .as_mut()
                    .filter(|w| w.token == token && !w.cancel.load(Ordering::Relaxed))
                {
                    if self.document.as_ref().is_some_and(|d| {
                        d.session == w.session && d.model.generation() == w.generation
                    }) {
                        w.answer = answer.map(|a| *a);
                    } else {
                        w.refused = Some("Çizim değişti. Kapsamı yeniden inceleyin.".into());
                    }
                }
            }
            Event::Apply => {
                let (Some(doc), Some(w)) = (
                    self.document.as_mut(),
                    self.blocks_panel.replacement.as_mut(),
                ) else {
                    return Task::none();
                };
                if doc.session != w.session || doc.model.generation() != w.generation {
                    w.refused =
                        Some("Çizim değişti. Pencereyi kapatıp kapsamı yeniden inceleyin.".into());
                    return Task::none();
                }
                let Some(a) = w.answer.as_ref().filter(|a| refusal(a).is_none()) else {
                    return Task::none();
                };
                let Some(plan) = a.plan_id.clone() else {
                    return Task::none();
                };
                if !matches!(a.result, CommandResult::Completed { .. }) {
                    return Task::none();
                }
                let done = kentos_native_application::call::call(
                    &mut doc.model,
                    None,
                    envelope(&w.input(), CallOp::Execute, Some(plan)),
                );
                if let Some(why) = refusal(&done) {
                    w.refused = Some(why.into());
                    return Task::none();
                }
                if matches!(done.result, CommandResult::Completed { .. }) {
                    self.block_replace_close();
                    self.say(
                        kentos_interaction::Level::Success,
                        "Blok yerleştirmeleri değiştirildi. Tek adımda geri alınır.",
                    );
                }
            }
        }
        Task::none()
    }
    pub(crate) fn block_replace_view(&self) -> Element<'_, Message> {
        let Some(w) = self.blocks_panel.replacement.as_ref() else {
            return text("").into();
        };
        let mut body=Column::new().spacing(12).push(label::body(format!("{} → {}",w.names.0,w.names.1)))
            .push(words::check(w.only_selected,"Yalnız seçili yerleştirmeler",(!w.selected.is_empty()).then(||msg(Event::Scope(!w.only_selected)))))
            .push(label::caption(format!("Açılıştaki seçim: {}. Kapsam: {} yerleştirme. Çizimde toplam {} doğrudan yerleştirme var.",w.selected.len(),if w.only_selected{w.selected.len()}else{w.all.len()},w.all.len())))
            .push(label::caption(format!("Tanımların içindeki {} yerleştirme değişmez. Eski tanım tutulur; dosya açılışında otomatik güncelleme yapılmaz.",w.nested)).style(style::text::muted));
        let mut apply = false;
        if let Some(why) = &w.refused {
            body = body.push(text(why).style(style::text::warning));
        } else if let Some(a) = &w.answer {
            if let Some(why) = refusal(a) {
                body = body.push(text(why).style(style::text::warning));
            } else if let Some(s) = &a.summary {
                body = body.push(label::body(format!(
                    "{} kayıt değişecek; bunların {} tanesi bağlı kayıt.",
                    s.counts.changed, s.counts.dependents
                )));
                for warn in &s.warnings {
                    body = body.push(words::text_line(words::Kind::Warn, warn.message.clone()));
                }
                let dropped = match &a.result {
                    CommandResult::Completed { output, .. } => {
                        output.get("dropped").and_then(|v| v.as_array())
                    }
                    _ => None,
                };
                if let Some(dropped) = dropped.filter(|v| !v.is_empty()) {
                    body = body.push(words::text_line(
                        words::Kind::Warn,
                        format!(
                            "Hedefte karşılığı olmayan öznitelikler düşecek: {}",
                            dropped
                                .iter()
                                .filter_map(|v| v.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    ));
                }
                if let CommandResult::Completed { output, .. } = &a.result {
                    if let Some(n) = output["places"].as_u64().filter(|n| *n > 0) {
                        body = body.push(words::text_line(words::Kind::Warn, format!(
                            "Hedefte taşınabilir karşılığı olmayan {n} özel öznitelik konumu kaldırılacak."
                        )));
                    }
                    if let Some(n) = output["fields"].as_u64().filter(|n| *n > 0) {
                        body = body.push(label::caption(format!(
                            "Hedefin {n} alan değeri yeniden hesaplanacak."
                        )));
                    }
                }
                apply = a.plan_id.is_some() && s.counts.changed > 0;
            }
        } else {
            body = body.push(label::body("Etki ayrı çalışma kopyasında inceleniyor…"));
        }
        body=body.push(label::caption("Uyumlu örnek değerleri korunur; hedef formülleri yeniden hesaplanır. Uyumsuz değer varsa hiçbir yerleştirme değiştirilmez. Kapat veya Esc çizimi korur.").style(style::text::muted));
        Frame::new(TITLE)
            .scroll(body)
            .action(words::secondary("Vazgeç", Some(msg(Event::Close))))
            .action(words::primary("Değiştir", apply.then(|| msg(Event::Apply))))
            .width(620.)
            .modal(msg(Event::Close))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_domain::Slot;
    use serde_json::json;

    fn app() -> (App, BlockId, BlockId) {
        let mut snapshot = kentos_kcad::decode(include_bytes!(
            "../../../../fixtures/kcad/v2/dynamic-blocks.kcad"
        ))
        .unwrap();
        let from = snapshot.blocks[0].id;
        let mut target = snapshot.blocks[0].clone();
        target.id = serde_json::from_value(json!("0192f5a1-2900-7000-8000-0000000000e1")).unwrap();
        target.name = "Yeni kapı".into();
        target.card.as_mut().unwrap().version = 2;
        let to = target.id;
        snapshot.blocks.push(target);
        let (mut app, _) = App::start(None, crate::settings::Settings::memory());
        app.document = Some(crate::document::Document::from_v2(snapshot, None).unwrap());
        app.selection.set([Slot(1)]);
        (app, from, to)
    }
    fn ready(app: &mut App) {
        let w = app.blocks_panel.replacement.as_ref().unwrap();
        let answer = kentos_native_application::call::call(
            &mut app.document.as_ref().unwrap().model.clone(),
            None,
            envelope(&w.input(), CallOp::Plan, None),
        );
        let _ = app.block_replace_event(Event::Ready(w.token, Some(Box::new(answer))));
    }
    fn bytes(app: &App) -> Vec<u8> {
        kentos_kcad::encode_verified(&app.document.as_ref().unwrap().model.to_snapshot_v2())
            .unwrap()
    }

    #[test]
    fn selected_scope_previews_without_writing_then_commits_the_plan_with_exact_single_undo() {
        let (mut a, from, to) = app();
        let before = bytes(&a);
        let _ = a.blocks_replace(from, to);
        let w = a.blocks_panel.replacement.as_ref().unwrap();
        assert!(w.only_selected);
        assert_eq!(w.selected.len(), 1);
        assert_eq!(w.all.len(), 3);
        ready(&mut a);
        assert_eq!(bytes(&a), before);
        assert_eq!(
            a.blocks_panel
                .replacement
                .as_ref()
                .unwrap()
                .answer
                .as_ref()
                .unwrap()
                .summary
                .as_ref()
                .unwrap()
                .counts
                .changed,
            1
        );
        let _ = a.block_replace_event(Event::Apply);
        assert!(a.blocks_panel.replacement.is_none());
        assert_eq!(
            a.document
                .as_ref()
                .unwrap()
                .model
                .entities()
                .filter(|e| matches!(e,Entity::Insert(i) if i.block==to))
                .count(),
            1
        );
        a.document.as_mut().unwrap().model.undo();
        assert_eq!(bytes(&a), before);
    }
    #[test]
    fn all_scope_is_explicit_and_keeps_frozen_uid_list_and_close_is_readonly() {
        let (mut a, from, to) = app();
        let before = bytes(&a);
        let _ = a.blocks_replace(from, to);
        let _ = a.block_replace_event(Event::Scope(false));
        ready(&mut a);
        assert_eq!(
            a.blocks_panel
                .replacement
                .as_ref()
                .unwrap()
                .input()
                .uids
                .as_ref()
                .unwrap()
                .len(),
            3
        );
        let cancel = Arc::clone(&a.blocks_panel.replacement.as_ref().unwrap().cancel);
        a.block_replace_close();
        assert!(cancel.load(Ordering::Relaxed));
        assert_eq!(bytes(&a), before);
        a.selection.clear();
        let _ = a.blocks_replace(from, to);
        assert!(!a.blocks_panel.replacement.as_ref().unwrap().only_selected);
        ready(&mut a);
        let _ = a.block_replace_event(Event::Apply);
        assert_eq!(
            a.document
                .as_ref()
                .unwrap()
                .model
                .entities()
                .filter(|e| matches!(e,Entity::Insert(i) if i.block==to))
                .count(),
            3
        );
        a.document.as_mut().unwrap().model.undo();
        assert_eq!(bytes(&a), before);
    }
    #[test]
    fn incompatible_plan_cannot_apply_and_late_results_do_not_replace_the_current_scope() {
        let (mut a, from, to) = app();
        let doc = &mut a.document.as_mut().unwrap().model;
        let p=serde_json::from_value(json!({"operation":"set","uids":[doc.uid(Slot(1)).unwrap().to_string()],"values":{"Genişlik":1.05}})).unwrap();
        kentos_native_application::block_params::execute(
            &mut kentos_native_application::ExecutionContext::new(doc),
            p,
        );
        let change =
            serde_json::from_value(json!({"operation":"dynamic","block":to,"dynamic":null}))
                .unwrap();
        kentos_native_application::blocks_edit::execute(
            &mut kentos_native_application::ExecutionContext::new(doc),
            change,
        );
        let before = bytes(&a);
        let _ = a.blocks_replace(from, to);
        let old = a.blocks_panel.replacement.as_ref().unwrap().token;
        let _ = a.block_replace_event(Event::Scope(false));
        let current = a.blocks_panel.replacement.as_ref().unwrap().token;
        assert_ne!(old, current);
        let _ = a.block_replace_event(Event::Ready(old, None));
        assert!(
            a.blocks_panel
                .replacement
                .as_ref()
                .unwrap()
                .answer
                .is_none()
        );
        ready(&mut a);
        assert!(
            refusal(
                a.blocks_panel
                    .replacement
                    .as_ref()
                    .unwrap()
                    .answer
                    .as_ref()
                    .unwrap()
            )
            .is_some()
        );
        let _ = a.block_replace_event(Event::Apply);
        assert_eq!(bytes(&a), before);
        assert!(a.blocks_panel.replacement.is_some());
    }
    #[test]
    fn document_changes_and_another_document_session_refuse_old_preview_without_mutation() {
        let (mut a, from, to) = app();
        let _ = a.blocks_replace(from, to);
        ready(&mut a);
        a.document.as_mut().unwrap().model.add(serde_json::from_value(json!({"kind":"line","id":0,"layerId":"kapilar","attrs":{},"a":{"x":0.,"y":0.},"b":{"x":1.,"y":0.}})).unwrap()).unwrap();
        let before = bytes(&a);
        let _ = a.block_replace_event(Event::Apply);
        assert_eq!(bytes(&a), before);
        assert!(
            a.blocks_panel
                .replacement
                .as_ref()
                .unwrap()
                .refused
                .is_some()
        );
        a.block_replace_close();
        let _ = a.blocks_replace(from, to);
        ready(&mut a);
        let snapshot = a.document.as_ref().unwrap().model.to_snapshot_v2();
        a.document = Some(crate::document::Document::from_v2(snapshot, None).unwrap());
        let before = bytes(&a);
        let _ = a.block_replace_event(Event::Apply);
        assert_eq!(bytes(&a), before);
        assert!(
            a.blocks_panel
                .replacement
                .as_ref()
                .unwrap()
                .refused
                .is_some()
        );
    }
}
