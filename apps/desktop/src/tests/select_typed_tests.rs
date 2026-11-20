//! UR2-062: `tool.select` typed in full, and its SEC, go back to selecting, idle or with a drawing command running,
//! with no word that the command is not on the desktop and no point added. Test code only.

use crate::app::{App, Message};
use crate::files_testing::app_with_drawing;

fn typed(app: &mut App, text: &str) {
    let _ = app.update(Message::CommandInput(text.into()));
    let _ = app.update(Message::CommandSubmitted);
}

#[test]
fn tool_select_typed_selects_without_a_platform_note() {
    for name in ["tool.select", "SEC"] {
        let mut app = app_with_drawing();
        let count = app.document.as_ref().expect("open").model.len();
        typed(&mut app, name);
        let _ = app.update(Message::Run("tool.polyline"));
        assert_eq!(app.session.tool_id(), "polyline");
        typed(&mut app, name);
        assert_eq!(app.session.tool_id(), "select", "{name}: {:?}", app.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>());
        assert!(!app.session.is_running(), "{name}");
        assert_eq!(app.document.as_ref().expect("open").model.len(), count, "{name}: nothing added");
        let said: Vec<&str> = app.log.lines().map(|l| l.text.as_str()).collect();
        assert!(!said.iter().any(|t| t.contains("taşınmadı") || t.contains("Geliştirme")), "{name}: {said:?}");
    }
}
