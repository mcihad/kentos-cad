//! Boru hattı denetimi without a utility network (the CBS review's ninth item's icon table, docs/adr/9337): it says why
//! and offers Ağlar… one press away, as Alt ağlar does, not only in the log.

use crate::app::Message;

#[test]
fn without_a_utility_network_the_check_offers_aglar() {
    let mut app = crate::files_testing::app_with_drawing();
    let _ = app.update(Message::Run("pipes.check"));
    let offered: Vec<(String, String)> = app
        .toasts
        .iter()
        .filter_map(|(_, toast, _)| toast.action_of().map(|(label, m)| (label.to_owned(), format!("{m:?}"))))
        .collect();
    assert!(
        offered.iter().any(|(label, m)| label == "Ağlar…" && m.contains("network.manage")),
        "{offered:?}"
    );
    assert!(crate::files_testing::last_said(&app).contains("altyapı ağı yok"));
}

/// The picture (`.run/gui-evidence/kullanim-9337/`): the notice with Ağlar….
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop-map pipes_prerequisite_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::drawing_fonts::load();
    crate::files_testing::usage_shots("9337", "04-boru-denetimi-aglar", || {
        let mut app = crate::files_testing::app_with_drawing();
        let _ = app.update(Message::Run("pipes.check"));
        app
    });
}
