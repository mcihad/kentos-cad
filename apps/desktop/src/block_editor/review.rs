//! Read-only editor impact from the existing command envelope's real dry run, plus the definition's reference graph.
//! No extra mutation rule or public protocol is introduced (CAD-ADV-06).
use std::collections::{BTreeMap, HashSet};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use super::{Event, msg};
use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use iced::{
    Element, Task,
    widget::{Column, row, text},
};
use kentos_commands::{
    BlocksEdit, CallEnvelope, CallOp, CommandResult, CommandSummary, Entity, TableSource,
};
use kentos_domain::Document;
use kentos_ui::{label, style, widget::Dialog as Frame};

pub(crate) const TITLE: &str = "Blok değişikliğinin etkisi";
static NEXT_REVIEW: AtomicU64 = AtomicU64::new(1);

pub(super) struct Pending {
    pub token: u64,
    pub cancel: Arc<AtomicBool>,
    pub generation: u64,
    pub session: u64,
    pub result: Option<Result<Review, String>>,
}

impl Drop for Pending {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Review {
    pub summary: CommandSummary,
    pub direct: usize,
    pub nested: usize,
    pub locked: usize,
    pub parents: Vec<String>,
    pub related: BTreeMap<String, usize>,
    pub pieces: (usize, usize),
}

fn stopped(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) {
        Err("İnceleme iptal edildi.".into())
    } else {
        Ok(())
    }
}

pub(super) fn compute(
    mut doc: Document,
    input: BlocksEdit,
    cancel: &AtomicBool,
) -> Result<Review, String> {
    stopped(cancel)?;
    let block = input.block.ok_or("Blok verilmedi.")?;
    let pieces_before = doc.block(block).map_or(0, |b| b.entities.len());
    let answer = kentos_native_application::call::call(
        &mut doc,
        None,
        CallEnvelope {
            command: kentos_commands::CAD_BLOCKS_EDIT.into(),
            version: None,
            op: CallOp::Plan,
            input: serde_json::to_value(input).map_err(|e| e.to_string())?,
            request_id: None,
            expected: None,
            relax_constraints: false,
        },
    );
    stopped(cancel)?;
    let output = match answer.result {
        CommandResult::Completed { output, .. } => output,
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => return Err(error.message),
        _ => return Err("İnceleme tamamlanmadı.".into()),
    };
    let summary = answer.summary.ok_or("Komut etki özeti üretmedi.")?;
    answer.plan_id.ok_or("Komut plan kimliği üretmedi.")?;
    let mut reached: HashSet<_> = summary.blocks.changed.iter().cloned().collect();
    // Count top-level placements once, even if several paths reach the same nested definition.
    loop {
        stopped(cancel)?;
        let mut changed = false;
        for b in doc.blocks() {
            if !reached.contains(&b.id.to_text())
                && b.entities
                    .iter()
                    .any(|e| matches!(e,Entity::Insert(i) if reached.contains(&i.block.to_text())))
            {
                reached.insert(b.id.to_text());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let parents: Vec<_> = doc
        .blocks()
        .iter()
        .filter(|b| b.id != block && reached.contains(&b.id.to_text()))
        .map(|b| b.name.clone())
        .collect();
    let mut direct = 0;
    let mut nested = 0;
    let mut locked = 0;
    let mut links = kentos_domain::copy_links::Copies::new();
    for (uid, e) in doc.entities_with_uids() {
        stopped(cancel)?;
        if let Entity::Insert(i) = e
            && reached.contains(&i.block.to_text())
        {
            if i.block == block {
                direct += 1;
            } else {
                nested += 1;
            }
            locked += usize::from(doc.layers().is_locked(&i.base.layer_id));
            links.insert(*uid.as_bytes(), *uid.as_bytes());
        }
    }
    let names: HashSet<_> = doc
        .blocks()
        .iter()
        .filter(|b| reached.contains(&b.id.to_text()))
        .map(|b| b.name.as_str())
        .collect();
    let mut related = BTreeMap::new();
    for e in doc.entities() {
        stopped(cancel)?;
        let reads_name = matches!(e,Entity::Table(t) if matches!(&t.source,Some(TableSource::Blocks{names:wanted,..}) if !names.is_empty() && (wanted.is_empty() || wanted.iter().any(|n|names.contains(n.as_str())))));
        if reads_name || kentos_domain::copy_links::links_to(e, &links) {
            let kind = match e {
                Entity::Dimension(_) => "Ölçü",
                Entity::Hatch(_) => "Tarama",
                Entity::Table(_) => "Tablo",
                Entity::Text(_) => "Bağlı yazı",
                Entity::Tag(_) => "Etiket",
                _ => "Diğer",
            };
            *related.entry(kind.to_owned()).or_default() += 1;
        }
    }
    let pieces_after = output["changed"]
        .as_array()
        .and_then(|a| {
            a.iter()
                .find(|b| b["id"].as_str() == Some(block.to_text().as_str()))
        })
        .and_then(|b| b["entities"].as_array())
        .map_or(pieces_before, Vec::len);
    Ok(Review {
        summary,
        direct,
        nested,
        locked,
        parents,
        related,
        pieces: (pieces_before, pieces_after),
    })
}

impl App {
    pub(super) fn block_editor_review(&mut self) -> Task<Message> {
        let (Some(draft), Some(editor)) = (self.document.as_ref(), self.block_editor.as_mut())
        else {
            return Task::none();
        };
        if editor.test.is_some() {
            self.warn("Önce test yerleştirmesinden düzenleyiciye dönün.");
            return Task::none();
        }
        if let Some(old) = editor.review.take() {
            old.cancel.store(true, Ordering::Relaxed);
        }
        let generation = draft.model.generation();
        let session = editor.original.session;
        let cancel = Arc::new(AtomicBool::new(false));
        let token = NEXT_REVIEW.fetch_add(1, Ordering::Relaxed);
        let input = super::update_input(draft, editor.block);
        let model = editor.original.model.clone();
        editor.review = Some(Pending {
            token,
            cancel: Arc::clone(&cancel),
            generation,
            session,
            result: None,
        });
        self.dialog = Some(Dialog::BlockReview);
        crate::db_connections::off_thread(
            move || compute(model, input, &cancel),
            move |result| {
                msg(Event::ReviewReady(
                    token,
                    session,
                    generation,
                    Box::new(result),
                ))
            },
        )
    }

    pub(super) fn block_editor_review_ready(
        &mut self,
        token: u64,
        session: u64,
        generation: u64,
        result: Result<Review, String>,
    ) {
        if self.dialog != Some(Dialog::BlockReview) {
            return;
        }
        let current_generation = self.document.as_ref().map(|d| d.model.generation());
        let Some(review) = self.block_editor.as_mut().and_then(|e| e.review.as_mut()) else {
            return;
        };
        if review.token == token
            && review.session == session
            && review.generation == generation
            && !review.cancel.load(Ordering::Relaxed)
        {
            review.result = Some(if current_generation == Some(generation) {
                result
            } else {
                Err("Taslak değişti. Pencereyi kapatıp etkiyi yeniden inceleyin.".into())
            });
        }
    }

    pub(crate) fn block_editor_review_close(&mut self) {
        if let Some(review) = self.block_editor.as_mut().and_then(|e| e.review.take()) {
            review.cancel.store(true, Ordering::Relaxed);
        }
        self.dialog = None;
    }

    pub(crate) fn block_editor_review_view(&self) -> Element<'_, Message> {
        let result = self
            .block_editor
            .as_ref()
            .and_then(|e| e.review.as_ref())
            .and_then(|r| r.result.as_ref());
        let mut body = Column::new().spacing(10);
        let line = |name: &str, value: String| -> Element<'_, Message> {
            row![
                label::caption(name.to_owned()).width(iced::FillPortion(2)),
                label::body(value).width(iced::FillPortion(3))
            ]
            .spacing(12)
            .into()
        };
        match result {
            None => {
                body=body.push(label::body("Değişiklik ayrı çalışma kopyasında inceleniyor…")).push(label::caption("Çizim ve taslak değiştirilmez; pencereyi kapatarak sonucu iptal edebilirsiniz.").style(style::text::muted));
            }
            Some(Err(why)) => {
                body = body.push(text(why.clone()).style(style::text::warning));
            }
            Some(Ok(r)) => {
                body = body
                    .push(line(
                        "Tanımdaki parça sayısı",
                        format!("{} → {}", r.pieces.0, r.pieces.1),
                    ))
                    .push(line("Doğrudan yerleştirmeler", r.direct.to_string()))
                    .push(line("İç içe kullanım üzerinden", r.nested.to_string()))
                    .push(line(
                        "Kilitli katmandaki yerleştirmeler",
                        r.locked.to_string(),
                    ));
                if !r.parents.is_empty() {
                    body = body.push(line("Tanımı içeren bloklar", r.parents.join(", ")));
                }
                let related = if r.related.is_empty() {
                    "Yok".into()
                } else {
                    r.related
                        .iter()
                        .map(|(kind, n)| format!("{n} {kind}"))
                        .collect::<Vec<_>>()
                        .join(" · ")
                };
                body = body
                    .push(line("Yerleştirmelere doğrudan bağlı", related))
                    .push(line(
                        "Komutun değiştireceği kayıtlar",
                        format!(
                            "{} değişir · {} eklenir · {} silinir",
                            r.summary.counts.changed,
                            r.summary.counts.added,
                            r.summary.counts.removed
                        ),
                    ))
                    .push(line(
                        "Bunların içindeki bağlı kayıtlar",
                        r.summary.counts.dependents.to_string(),
                    ));
                for warning in &r.summary.warnings {
                    body = body.push(text(warning.message.clone()).style(style::text::warning));
                }
                body=body.push(label::caption("Yerleştirmeler tanımı izler; kendi kayıtları değişmese de görünümleri değişebilir. Kilitli katman tanım değişikliğini yalıtmaz. Bağlı öğe sayısı, değerinin mutlaka değişeceği anlamına gelmez.").style(style::text::muted))
                    .push(label::caption("Bu pencere hiçbir şey uygulamaz. Taslağı Kaydet ile güncellersiniz; Vazgeç eski çizimi korur. İç içe öznitelik değerleri için otomatik eşitleme eklenmez.").style(style::text::muted));
            }
        }
        Frame::new(TITLE)
            .scroll(body)
            .action(words::primary("Kapat", Some(msg(Event::CloseReview))))
            .width(600.)
            .modal(msg(Event::CloseReview))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_scenes::{editing, handle};
    use serde_json::json;

    fn model() -> Document {
        Document::from_snapshot_v2(
            kentos_kcad::decode(include_bytes!(
                "../../../../fixtures/kcad/v2/dynamic-blocks.kcad"
            ))
            .unwrap(),
        )
        .unwrap()
    }

    fn changed(doc: &Document) -> BlocksEdit {
        let block = &doc.blocks()[0];
        let mut input = kentos_interaction::blocks::input(
            kentos_commands::BlockEditOperation::Update,
            Some(block.id),
        );
        let mut pieces = block.entities.clone();
        pieces.push(
            serde_json::from_value(json!({
                "kind":"line","id":5,"layerId":"","attrs":{},
                "a":{"x":0.06,"y":0.45},"b":{"x":0.3,"y":0.45}
            }))
            .unwrap(),
        );
        input.entities = Some(pieces);
        input
    }

    #[test]
    fn real_plan_is_read_only_and_separates_render_reach_from_changed_records() {
        let doc = model();
        let before = doc.to_snapshot_v2();
        let generation = doc.generation();
        let journal: Vec<_> = doc.journal().cloned().collect();
        let input = changed(&doc);
        let r = compute(doc.clone(), input.clone(), &AtomicBool::new(false)).unwrap();
        assert_eq!((r.direct, r.nested, r.locked, r.pieces), (3, 0, 0, (4, 5)));
        assert!(r.related.is_empty());
        assert_eq!(r.summary.blocks.changed, vec![doc.blocks()[0].id.to_text()]);
        assert_eq!(
            r.summary.counts.changed, 0,
            "references render the new definition without rewriting each insert"
        );
        let mut actual = doc.clone();
        let done = kentos_native_application::call::call(
            &mut actual,
            None,
            CallEnvelope {
                command: kentos_commands::CAD_BLOCKS_EDIT.into(),
                version: None,
                op: CallOp::Execute,
                input: serde_json::to_value(input).unwrap(),
                request_id: None,
                expected: None,
                relax_constraints: false,
            },
        );
        assert!(matches!(done.result, CommandResult::Completed { .. }));
        assert_eq!(done.summary.unwrap().counts, r.summary.counts);
        assert_eq!(actual.blocks()[0].entities.len(), 5);
        assert_eq!(doc.to_snapshot_v2(), before);
        assert_eq!(doc.generation(), generation);
        assert_eq!(doc.journal().cloned().collect::<Vec<_>>(), journal);
    }

    #[test]
    fn unchanged_definition_has_no_render_reach_and_cancellation_refusal_are_read_only() {
        let doc = model();
        let before = doc.to_snapshot_v2();
        let mut input = changed(&doc);
        input.entities = Some(doc.blocks()[0].entities.clone());
        let r = compute(doc.clone(), input, &AtomicBool::new(false)).unwrap();
        assert_eq!((r.direct, r.nested, r.pieces), (0, 0, (4, 4)));
        assert!(r.summary.blocks.changed.is_empty());
        assert_eq!(
            compute(doc.clone(), changed(&doc), &AtomicBool::new(true)).unwrap_err(),
            "İnceleme iptal edildi."
        );
        let mut invalid = changed(&doc);
        invalid.entities.as_mut().unwrap().push(
            serde_json::from_value(json!({
                "kind":"insert","id":6,"layerId":"","attrs":{},"block":doc.blocks()[0].id,
                "p":{"x":0.,"y":0.},"scale":1.,"rotation":0.
            }))
            .unwrap(),
        );
        assert!(compute(doc.clone(), invalid, &AtomicBool::new(false)).is_err());
        assert_eq!(doc.to_snapshot_v2(), before);
    }

    #[test]
    fn nested_paths_are_counted_once_and_locked_and_linked_references_are_visible() {
        let mut snapshot = model().to_snapshot_v2();
        let child = snapshot.blocks[0].id;
        let parent = "0192f5a1-2900-7000-8000-0000000000e1";
        let grand = "0192f5a1-2900-7000-8000-0000000000e2";
        let insert = |block: serde_json::Value, id: u32| {
            json!({
                "kind":"insert","id":id,"layerId":"","attrs":{},"block":block,
                "p":{"x":0.,"y":0.},"scale":1.,"rotation":0.
            })
        };
        for (id, name, pieces) in [
            (
                parent,
                "İki kapı",
                vec![insert(json!(child), 1), insert(json!(child), 2)],
            ),
            (grand, "Giriş", vec![insert(json!(parent), 1)]),
        ] {
            snapshot.blocks.push(
                serde_json::from_value(json!({
                    "id":id,"name":name,"base":{"x":0.,"y":0.},"units":"m","entities":pieces
                }))
                .unwrap(),
            );
        }
        snapshot
            .layers
            .iter_mut()
            .find(|l| l.id == "kapilar")
            .unwrap()
            .locked = true;
        let uid = snapshot.uids[0];
        let mut doc = Document::from_snapshot_v2(snapshot).unwrap();
        for id in [parent, grand] {
            let mut e = insert(json!(id), 0);
            e["layerId"] = json!("kapilar");
            doc.add(serde_json::from_value(e).unwrap()).unwrap();
        }
        doc.add(
            serde_json::from_value(json!({
                "kind":"dimension","id":0,"layerId":"duvar","attrs":{},
                "a":{"x":0.,"y":0.},"b":{"x":0.,"y":0.9},"offset":0.2,"height":0.1,
                "refs":{"a":{"uid":uid,"path":[1],"place":{"kind":"start"}},
                        "b":{"uid":uid,"path":[1],"place":{"kind":"end"}}}
            }))
            .unwrap(),
        )
        .unwrap();
        for names in [vec![], vec!["İki kapı"], vec!["Giriş"], vec!["İlgisiz"]] {
            doc.add(
                serde_json::from_value(json!({
                    "kind":"table","id":0,"layerId":"duvar","attrs":{},
                    "p":{"x":10.,"y":10.},"rotation":0.,"height":0.1,
                    "rows":[0.2],"columns":[1.],"cells":[["Ad"]],
                    "source":{"kind":"blocks","names":names,"columns":[]}
                }))
                .unwrap(),
            )
            .unwrap();
        }
        let before = doc.to_snapshot_v2();
        let r = compute(doc.clone(), changed(&doc), &AtomicBool::new(false)).unwrap();
        assert_eq!(
            (r.direct, r.nested),
            (3, 2),
            "two paths within one parent are one top-level placement"
        );
        assert_eq!(r.locked, 5);
        assert_eq!(r.parents, vec!["İki kapı", "Giriş"]);
        assert_eq!(r.related.get("Ölçü"), Some(&1));
        assert_eq!(r.related.get("Tablo"), Some(&3));
        assert_eq!(doc.to_snapshot_v2(), before);
    }

    #[test]
    fn a_closed_or_replaced_review_cannot_accept_a_late_result_and_a_changed_draft_is_explained() {
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
        let draft = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let _ = app.block_editor_review();
        let pending = app.block_editor.as_ref().unwrap().review.as_ref().unwrap();
        let (token, session, generation, cancel) = (
            pending.token,
            pending.session,
            pending.generation,
            pending.cancel.clone(),
        );
        app.block_editor_review_close();
        assert!(cancel.load(Ordering::Relaxed));
        app.block_editor_review_ready(token, session, generation, Err("old".into()));
        assert!(app.block_editor.as_ref().unwrap().review.is_none());
        let _ = app.block_editor_review();
        app.block_editor_review_ready(token, session, generation, Err("old".into()));
        assert!(
            app.block_editor
                .as_ref()
                .unwrap()
                .review
                .as_ref()
                .unwrap()
                .result
                .is_none()
        );
        let new_token = app
            .block_editor
            .as_ref()
            .unwrap()
            .review
            .as_ref()
            .unwrap()
            .token;
        assert_ne!(new_token, token);
        assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(), draft);
        handle(&mut app); // another command, e.g. from the AI session, while the result is pending
        app.block_editor_review_ready(
            new_token,
            session,
            generation,
            Err("obsolete failure".into()),
        );
        let pending = app.block_editor.as_ref().unwrap().review.as_ref().unwrap();
        assert!(
            pending
                .result
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap_err()
                .contains("Taslak değişti")
        );
        app.block_editor_review_close();
        assert_eq!(
            app.block_editor
                .as_ref()
                .unwrap()
                .original
                .model
                .to_snapshot_v2(),
            original
        );
        let _ = app.update(msg(Event::Discard));
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            original
        );
    }
}
