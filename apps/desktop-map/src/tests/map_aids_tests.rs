//! KentOS Harita's drawing aids (docs/adr/9185): its program has the overlap lamp's options that KentOS Mimari's has not
//! (architecture_ribbon_tests.rs keeps Mimari's half; status.rs has the menu).

use crate::files_testing::{app_with_drawing, make_cad};

#[test]
fn kentos_harita_s_aids_have_the_overlap_options() {
    let mut app = app_with_drawing();
    make_cad(&mut app);
    app.program = Some(kentos_launch::Module::Map);
    let text = format!("{:?}", app.compact_aids_menu());
    assert!(text.contains("Çakışma seçenekleri"), "{text}");
}
