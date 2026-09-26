//! What the tests of opening, saving and recovery share (opening.rs,
//! saving.rs, recovery.rs): the app's own tasks driven to their end without a
//! window, scratch folders under the system's temporary one (never the user's
//! files), and drawings to save and open. Test code only.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use iced::Task;
use iced::futures::StreamExt as _;
use kentos_contracts::{DocumentSnapshotV1, Entity, EntityBase, EntityId, PointEntity, Vec2};

use crate::app::{App, Message};
use crate::document::Document;

/// Runs a task of the app to its end: its messages go back to the app, whose tasks run too.
pub fn drive(app: &mut App, task: Task<Message>) {
    let Some(mut stream) = iced_runtime::task::into_stream(task) else {
        return;
    };
    while let Some(action) = iced::futures::executor::block_on(stream.next()) {
        if let iced_runtime::Action::Output(message) = action {
            let next = app.update(message);
            drive(app, next);
        }
    }
}

/// A fresh folder under the system's temporary one.
pub fn scratch(name: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "kentos-desktop-{name}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temporary directory");
    dir
}

const DEMO: &str = include_str!("../../../fixtures/document/v1/sample.json");

/// The web's sample drawing (13 objects) with `extra` points more, as a new drawing.
pub fn drawing(extra: usize) -> Document {
    let mut snapshot = DocumentSnapshotV1::from_json(DEMO).expect("reads");
    let layer = "cizim".to_owned();
    let first = snapshot
        .entities
        .iter()
        .map(|e| e.base().id)
        .max()
        .unwrap_or(0);
    for i in 0..extra {
        snapshot.entities.push(Entity::Point(PointEntity {
            base: EntityBase {
                id: first + i as u32 + 1,
                layer_id: layer.clone(),
                color: None,
                attrs: [("Ad".to_owned(), format!("N{i}"))].into(),
                label: None,
                symbol: None,
            },
            p: Vec2 {
                x: 486_500.0 + i as f64,
                y: 4_420_200.0,
            },
            z: None,
        }));
    }
    let mut doc = Document::new(snapshot, None).expect("opens");
    // As a drawing saved before: persistent ids of its own, no v1 source.
    let mut v2 = doc.model.to_snapshot_v2();
    v2.migrated_from = None;
    for (i, uid) in v2.uids.iter_mut().enumerate() {
        let mut id = [0x42u8; 16];
        id[8..].copy_from_slice(&(i as u64 + 1).to_be_bytes());
        *uid = EntityId(id);
    }
    doc = Document::from_v2(v2, None).expect("opens");
    doc
}

/// The sample drawing saved as a `.kcad` v2 file in `dir`.
pub fn saved(dir: &std::path::Path, name: &str, extra: usize) -> PathBuf {
    let path = dir.join(name);
    crate::document::write(&drawing(extra).model.to_snapshot_v2(), &path).expect("writes");
    path
}

/// The app with the sample drawing on screen (no file), in memory.
pub fn app_with_drawing() -> App {
    let (mut app, _) = App::boot(None);
    let _ = app.update(Message::Opened(Some(Ok(Box::new(drawing(0))))));
    app
}

/// What the command line said last.
pub fn last_said(app: &App) -> String {
    use kentos_ui::widget::command_line::Entry;
    match app.history.last() {
        Some(Entry::Input(t) | Entry::Value(t) | Entry::Output(t) | Entry::Warning(t) | Entry::Error(t)) => t.clone(),
        _ => String::new(),
    }
}
