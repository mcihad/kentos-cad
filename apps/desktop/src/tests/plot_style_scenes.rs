//! Pictures of Baskı stili tablosu (docs/adr/1130; CAD-ADV-16) for the owner to look at, in tools_screens' harness:
//! a CTB whose every unmapped field is named, its pen 1 given to Duvar and the profile applied to A-101; a named STB.
//!
//! ```text
//! KENTOS_SHOTS_ONLY=baski-stili-ctb,baski-stili-stb cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```

use crate::app::{App, Message};
use crate::plot_styles_window::Event as P;
use crate::tools_screens::Scene;

/// The drawing of `cad.plotstyles.import`'s shared cases (layers Duvar and Aks, sheets A-101 and A-102), opened.
fn open(app: &mut App) {
    let fx: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.plotstyles.import.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(fx["setup"].clone()).expect("a drawing");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    crate::files_testing::make_cad(app);
}

fn table(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/commands/v1/plotstyles").join(name)
}

fn ctb(app: &mut App) {
    open(app);
    let _ = app.run("sheet.plotStyles");
    let _ = app.update(Message::PlotStyles(P::Picked(Some(table("kalemler.ctb")))));
    let duvar = app.plot_styles.as_ref().and_then(|w| w.pens.iter().position(|p| p.0 == "duvar")).unwrap_or(0);
    let _ = app.update(Message::PlotStyles(P::Pen(duvar, 19)));
    let _ = app.update(Message::PlotStyles(P::Sheet(0, true)));
}

fn stb(app: &mut App) {
    open(app);
    let _ = app.run("sheet.plotStyles");
    let _ = app.update(Message::PlotStyles(P::Picked(Some(table("stiller.stb")))));
    let aks = app.plot_styles.as_ref().and_then(|w| w.pens.iter().position(|p| p.0 == "aks")).unwrap_or(0);
    let _ = app.update(Message::PlotStyles(P::Pen(aks, 2)));
    let _ = app.update(Message::PlotStyles(P::View(crate::plot_styles_window::View::Styles)));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![("baski-stili-ctb", ctb), ("baski-stili-stb", stb)]
}
