//! The KentOS wordmark where the app draws it (kentos_ui's brand.rs): the start page's, the ribbon's and Hakkında's,
//! each from the picture made once for its size and the screen's device pixels. Pictures only, by hand.

use crate::app::{App, Message};
use kentos_launch::Module;

/// `.run/gui-evidence/logo-boyutlari/`, at one and two device pixels each:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib brand_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/logo-boyutlari");
    std::fs::create_dir_all(&out).expect("a folder");
    for (scale, theme) in [(1.0_f32, "light"), (2.0, "dark"), (2.0, "light")] {
        kentos_ui::widget::brand::set_device_scale(scale);
        let size = Size::new(1280.0, 800.0);
        let (mut app, _) = App::boot(None);
        app.program = Some(Module::Map);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        app.open_start();
        let mut snapshot = Snapshot::new(size).expect("a renderer").scale(scale);
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-baslangic-{theme}-{scale}x.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        let _ = app.update(Message::Run("help.about"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-hakkinda-{theme}-{scale}x.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
    kentos_ui::widget::brand::set_device_scale(2.0);
}
