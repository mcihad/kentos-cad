//! The log in the app (docs/adr/0114): the status bar's message, what the
//! command line's ↑ brings back, the lists' lines and their pictures.

use std::time::{Duration, Instant};

use kentos_interaction::Level;
use kentos_ui::widget::command_line::Entry;

use crate::app::{App, Message};
use crate::bottom::BottomTab;
use crate::files_testing::app_with_drawing;

const MS: fn(u64) -> Duration = Duration::from_millis;

#[test]
fn the_status_bar_shows_the_newest_line_it_shows_for_its_seconds() {
    let mut app = app_with_drawing();
    // Well after the opening's own message has gone.
    let now = Instant::now() + Duration::from_secs(60);
    app.say(Level::Info, "Çizgi: 12.500 m");
    // A line that goes on the one before it is not shown (the web's `flashOf`).
    app.say(Level::Info, "  Y 10.000  X 5.000");
    let _ = app.follow_log(now);
    let flash = app.follow.flash.clone().expect("a message");
    assert_eq!(
        (flash.text.as_str(), flash.icon),
        ("Çizgi: 12.500 m", "info")
    );
    // It fades in, stays 5 s, then fades out.
    assert_eq!(flash.alpha(now), 0.0);
    assert!((flash.alpha(now + MS(80)) - 0.5).abs() < 1e-3);
    assert_eq!(flash.alpha(now + MS(1000)), 1.0);
    assert!((flash.alpha(now + MS(5080)) - 0.5).abs() < 1e-3);
    assert_eq!(flash.alpha(now + MS(5160)), 0.0);
    assert!(flash.fading(now + MS(10)) && !flash.fading(now + MS(1000)));
    assert!(flash.fading(now + MS(5010)) && !flash.fading(now + MS(5200)));

    // A command's line never takes its place.
    app.say(Level::Command, "Çizgi");
    let _ = app.follow_log(now + MS(1000));
    assert_eq!(
        app.follow.flash.as_ref().map(|f| f.text.as_str()),
        Some("Çizgi: 12.500 m")
    );

    // A warning takes it at once (the cell stays visible) and stays 9 s.
    let later = now + MS(1000);
    app.warn("“yollar.shp”: 3 nesne alınmadı.");
    let _ = app.follow_log(later);
    let flash = app.follow.flash.clone().expect("the warning");
    assert_eq!(flash.icon, "warning");
    assert_eq!(flash.alpha(later), 1.0);
    assert_eq!(flash.alpha(later + MS(8990)), 1.0);
    assert_eq!(flash.alpha(later + MS(9200)), 0.0);
    // Gone, it is dropped at the next frame.
    app.log_frame(later + MS(9200));
    assert!(app.follow.flash.is_none());
}

#[test]
fn several_lines_at_once_leave_the_newest_the_bar_shows() {
    let mut app = app_with_drawing();
    let now = Instant::now() + Duration::from_secs(60);
    app.error("Bir hata.");
    app.say(Level::Success, "Kaydedildi.");
    app.say(Level::Command, "› 12");
    let _ = app.follow_log(now);
    let flash = app.follow.flash.as_ref().expect("a message");
    assert_eq!(
        (flash.text.as_str(), flash.icon),
        ("Kaydedildi.", "success")
    );
    // Nothing new: it stays as it is.
    let _ = app.follow_log(now + MS(500));
    assert_eq!(
        app.follow.flash.as_ref().map(|f| f.text.as_str()),
        Some("Kaydedildi.")
    );
}

#[test]
fn what_is_typed_in_the_command_line_comes_back_with_its_arrow() {
    let mut app = app_with_drawing();
    let _ = app.update(Message::CommandRun("L".into()));
    let _ = app.update(Message::CommandInput("0,0".into()));
    let _ = app.update(Message::CommandSubmitted);
    // Typed twice in a row: kept once, as the web's `remember`.
    let _ = app.update(Message::CommandInput("0,0".into()));
    let _ = app.update(Message::CommandSubmitted);
    assert_eq!(
        app.typed,
        [Entry::Input("L".into()), Entry::Input("0,0".into())]
    );
    // The history has the tool's name and what was typed, with the web's mark.
    let commands: Vec<&str> = app
        .log
        .lines()
        .filter(|l| l.level == Level::Command)
        .map(|l| l.text.as_str())
        .collect();
    assert!(
        commands.ends_with(&["Çizgi", "› 0,0", "› 0,0"]),
        "{commands:?}"
    );
}

#[test]
fn the_command_line_is_one_row_and_its_button_opens_the_panel() {
    let mut app = app_with_drawing();
    assert!(!app.command_expanded);
    let _ = app.update(Message::CommandHistoryToggled);
    assert!(app.command_expanded);
    assert_eq!(app.bottom_tab, BottomTab::History);
    let _ = app.view();
    let _ = app.update(Message::BottomTab(BottomTab::Messages));
    let _ = app.update(Message::CommandHistoryToggled);
    assert!(!app.command_expanded);
    // Opened again: the tab it was on.
    let _ = app.update(Message::CommandHistoryToggled);
    assert_eq!(app.bottom_tab, BottomTab::Messages);
    let _ = app.view();
}

/// The log as the web's pictures show it (apps/web/scripts/e2e/out/layout,
/// the bottom panel and the status bar): Komut geçmişi with every level and
/// the Uyarılar badge, Uyarılar, and the closed panel with a long warning in
/// the status bar, dark and light, at 1440×900 and 1100×650;
/// `.run/shots/gunluk-*`:
///
/// ```text
/// cargo test -p kentos-desktop message_log_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for scene in ["gecmis", "uyarilar", "kapali"] {
                let mut app = app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let _ = app.update(Message::CommandRun("L".into()));
                let _ = app.update(Message::CommandInput("486950,4419990".into()));
                let _ = app.update(Message::CommandSubmitted);
                let _ = app.update(Message::CommandInput("@12.5<100".into()));
                let _ = app.update(Message::CommandSubmitted);
                let _ = app.update(Message::Run("tool.cancel"));
                app.say(Level::Success, "Kaydedildi: Örnek pafta.kcad (13 nesne).");
                app.warn("“yollar.shp” içinde alınmayanlar: Kısa halka: 1, üçten az köşesi var; alınmadı.");
                app.error(
                    "“ABC” adında bir komut yok. Tüm komutlar ve kısayollar için F1’e basın.",
                );
                match scene {
                    "gecmis" => {
                        let _ = app.update(Message::BottomTab(BottomTab::History));
                    }
                    "uyarilar" => {
                        let _ = app.update(Message::BottomTab(BottomTab::Messages));
                    }
                    _ => {
                        app.warn(
                            "“parseller.dxf” içinde alınmayanlar: 3 blok, 2 dış başvuru, 1 çok satırlı yazı; blokların içindekiler ayrı nesne olarak alınabilir.",
                        );
                    }
                }
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                app.follow.show_fully();
                let file = out.join(format!("gunluk-{scene}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
