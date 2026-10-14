//! The pictures of docs/adr/0282 (AutoCAD takma adları): Uygulama ayarları › Görünüm with Komut satırı's switch on;
//! the command line with `JOG` typed, its row Ölçülendirme › Kırıklı yarıçap; `LA` typed, Katman ve öznitelik paneli
//! first; the shortcuts window's AutoCAD group. `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=autocad-ayar,autocad-satir-jog,autocad-satir-la,autocad-kisayollar cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use iced::advanced::widget::operation;
use kentos_ui::snapshot::Snapshot;

use crate::app::{App, COMMAND_INPUT, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::{Objects, open};
use crate::tools_screens::Pointed;

/// A CAD drawing with the profile on.
fn with_profile(app: &mut App) {
    open(app, Objects::new());
    make_cad(app);
    let _ = app
        .settings
        .choose(&[(crate::alias_profile::SETTING, serde_json::Value::Bool(true))]);
    app.apply_settings();
}

fn settings(app: &mut App) {
    with_profile(app);
    let _ = app.update(Message::Run("tools.options"));
}

fn typed_jog(app: &mut App) {
    with_profile(app);
    app.command_input = "JOG".into();
}

fn typed_la(app: &mut App) {
    with_profile(app);
    app.command_input = "LA".into();
}

fn shortcuts(app: &mut App) {
    with_profile(app);
    let _ = app.update(Message::Run("help.shortcuts"));
}

/// Every scrollable at its end: the window's last groups in sight.
fn to_end(s: &mut Snapshot, app: &mut App) {
    s.operate(app.view(), Box::new(crate::files_testing::SnapAll));
}

/// The command line focused: its list open over what is typed.
fn focus_line(s: &mut Snapshot, app: &mut App) {
    s.operate(
        app.view(),
        Box::new(operation::focusable::focus(COMMAND_INPUT.into())),
    );
}

pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        ("autocad-ayar", settings, to_end),
        ("autocad-satir-jog", typed_jog, focus_line),
        ("autocad-satir-la", typed_la, focus_line),
        ("autocad-kisayollar", shortcuts, to_end),
    ]
}
