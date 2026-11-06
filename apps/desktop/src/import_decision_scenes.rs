//! The picture of docs/adr/1010 (the reader's decisions in one section): ezdxf's decisions file
//! (`fixtures/exchange/v1/decisions.dxf`, millimetres, layers off/frozen/missing/unused, a mirrored and a tilted
//! plane, a block 3 wide and half high, one sheet) in Çizim al with Blokları patlat on: Okuma kararları by topic,
//! then the other notes. `KENTOS_SHOTS_ONLY=okuma-kararlari cargo test -p kentos-desktop --lib tools_screens::screens
//! -- --ignored --nocapture`.
//!
//! Test code only.

use crate::app::{App, Message, Picker};
use crate::exchange::{Event, drawing_import};
use crate::tools_screens::Scene;

fn decisions(app: &mut App) {
    crate::files_testing::make_cad(app);
    app.picker = Picker::File(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/exchange/v1/decisions.dxf"),
    );
    let task = app.run("file.import.dxf");
    crate::files_testing::drive(app, task);
    let task = app.update(Message::Exchange(Box::new(Event::DrawingImport(drawing_import::Event::Explode))));
    crate::files_testing::drive(app, task);
}

/// docs/adr/1128: a file whose ACAD_TABLE follows another program's table style (`table-styles.dxf`): the Stil
/// decisions, what of the style KentOS cannot keep, and the style no table follows, left out.
/// `KENTOS_SHOTS_ONLY=okuma-tablo-stili`.
fn table_styles(app: &mut App) {
    crate::files_testing::make_cad(app);
    app.picker = Picker::File(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/exchange/v1/table-styles.dxf"),
    );
    let task = app.run("file.import.dxf");
    crate::files_testing::drive(app, task);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![("okuma-kararlari", decisions), ("okuma-tablo-stili", table_styles)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_import_window_lists_the_decisions_once() {
        let mut app = crate::files_testing::app_with_drawing();
        decisions(&mut app);
        let Some(crate::exchange::Window::DrawingImport(s)) = &app.exchange else { panic!("the window is open") };
        let r = s.read_result().expect("read");
        let codes: Vec<&str> = r.report.decisions.iter().map(|d| d.code.as_str()).collect();
        assert!(codes.contains(&"block_nonuniform_arcs_nurbs"), "exploded: {codes:?}");
        assert!(codes.contains(&"layer_unused_left_out"));
        let notes = crate::exchange::words::notes_beside(&r.report.notes, &r.report.decisions);
        assert!(!notes.iter().any(|n| n.what == "Birim"), "the unit's note is said once, as a decision");
        let line = crate::exchange::words::decision_text(&r.report.decisions[0]);
        assert_eq!(line, "Birim: dosya milimetre biriminde; değerler çizimin birimine, metreye çevrildi");
    }
}
