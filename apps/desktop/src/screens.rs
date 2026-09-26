//! Pictures of the windows this slice added (docs/adr/0030), for the owner to
//! look at: the open's window, a stopped open, the save's panel and its
//! failures, the recovery offer and a restored copy, in the dark and the
//! light theme. Not a test of correctness and not run by default; writes PNGs
//! to the worktree's `.run/screens` (never committed):
//!
//! ```text
//! cargo test -p kentos-desktop screens -- --ignored --nocapture
//! ```

use std::path::Path;
use std::time::Duration;

use iced::Size;
use kentos_ui::snapshot::Snapshot;

use crate::app::{App, Message, Picker};
use crate::files_testing::{app_with_drawing, drive, saved, scratch};
use crate::opening::{Event as Open, Purpose, Stage};
use crate::recovery::{Answer, Event as Rec, Recovery};
use crate::saving::{Event as Save, Fault, Faults};
use crate::settings::Settings;

fn theme(app: &mut App, name: &str) {
    let _ = app
        .settings
        .choose(&[("appearance.theme", serde_json::Value::from(name))]);
    app.apply_settings();
    app.command_expanded = true;
}

fn picture(app: &mut App, out: &Path, name: &str) {
    let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    snapshot.settle(app, App::view, &mut update);
    let _ = snapshot.render(app.view(), &app.theme());
    let file = out.join(format!("{name}.png"));
    snapshot
        .render(app.view(), &app.theme())
        .save(&file)
        .expect("writes the picture");
    println!("{}", file.display());
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/screens");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let dir = scratch("screens");
    let big = saved(&dir, "Ada 101.kcad", 200_000);
    for mode in ["dark", "light"] {
        // 1. The open's window, mid-way; 2. stopped: the drawing on screen as it was.
        let mut app = app_with_drawing();
        theme(&mut app, mode);
        let task = app.start_opening(big.clone(), Purpose::File);
        let id = app.opening.as_ref().expect("opening").id;
        std::thread::sleep(Duration::from_millis(300));
        for stage in [
            Stage::Project {
                name: "Ada 101".into(),
                layers: 1,
                objects: 200_013,
            },
            Stage::Reading {
                done: 118_784,
                total: 200_013,
            },
        ] {
            let _ = app.update(Message::Opening(Open::Progress { id, stage }));
        }
        picture(&mut app, &out, &format!("desktop-{mode}-1-open-progress"));
        let _ = app.update(Message::Opening(Open::Cancel));
        drive(&mut app, task);
        picture(&mut app, &out, &format!("desktop-{mode}-2-open-stopped"));

        // 3. The save's panel while it runs, then a full disk and a refused permission.
        let layer = app.document.as_ref().expect("a drawing").layers()[0]
            .id
            .clone();
        let _ = app.update(Message::LayerLocked(layer));
        let target = dir.join(format!("pafta-{mode}.kcad"));
        let task = app.start_saving(target.clone());
        std::thread::sleep(Duration::from_millis(350));
        let id = app.saving.as_ref().expect("saving").id;
        let _ = app.update(Message::Saving(Save::Progress {
            id,
            stage: crate::saving::Stage::Writing {
                done: 41_000_000,
                total: 96_600_000,
            },
        }));
        picture(&mut app, &out, &format!("desktop-{mode}-3-save-panel"));
        drive(&mut app, task);
        app.picker = Picker::File(target);
        let layer = app.document.as_ref().expect("a drawing").layers()[0]
            .id
            .clone();
        let _ = app.update(Message::LayerLocked(layer));
        app.save_faults = Faults {
            fail: Some((Fault::Write, std::io::ErrorKind::StorageFull)),
        };
        let task = app.update(Message::Run("file.save"));
        drive(&mut app, task);
        app.save_faults = Faults {
            fail: Some((Fault::Rename, std::io::ErrorKind::PermissionDenied)),
        };
        let task = app.update(Message::Run("file.save"));
        drive(&mut app, task);
        picture(&mut app, &out, &format!("desktop-{mode}-4-save-failures"));

        // 5. The recovery offer a crashed KentOS leaves; 6. the copy restored, unsaved and without a file.
        let root = scratch("screens-recovery");
        let mut crashed = app_with_drawing();
        crashed.recovery = Recovery::open(&root).expect("a folder");
        let layer = crashed.document.as_ref().expect("a drawing").layers()[0]
            .id
            .clone();
        let _ = crashed.update(Message::LayerLocked(layer));
        let now = std::time::Instant::now();
        let first = crashed.recovery_due(now);
        drive(&mut crashed, first);
        let task = crashed.recovery_due(now + Duration::from_secs(4));
        drive(&mut crashed, task);
        drop(crashed);
        let (mut next, _) = App::start_with(
            None,
            Settings::memory(),
            Recovery::open(&root).expect("a folder"),
        );
        theme(&mut next, mode);
        picture(&mut next, &out, &format!("desktop-{mode}-5-recovery-offer"));
        let task = next.update(Message::Recovery(Rec::Answer(Answer::Restore)));
        drive(&mut next, task);
        picture(&mut next, &out, &format!("desktop-{mode}-6-recovered"));
        let _ = std::fs::remove_dir_all(&root);
    }
    let _ = std::fs::remove_dir_all(&dir);
}
