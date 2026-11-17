//! A late answer does not reach a panel opened again (plan/eklenti.md ADIM 22 › Tanım ve oturum, PY-UI-17: "pencere
//! kapanması, kaldırma ve belge değişimi geç geri çağırmayı etkisiz kılar"; docs/adr/8057 §117): a panel closed while
//! its command ran and opened again is a new panel; the old run's rows and its `ctx.panel` calls do not reach it.

use serde_json::{Value, json};

use super::super::panels::{OpenPanel, PanelCall, PanelOrigin};
use super::demo::vitrin_package;
use super::*;
use crate::app::App;

const VITRIN: &str = "tr.ornek.vitrin";

fn definition() -> kentos_extensions::ui_panel::PanelDefinition {
    kentos_extensions::ui_panel::read(&json!({
        "format": "kentos.ui.panel", "schemaVersion": 1, "title": "Liste",
        "body": [
            { "type": "button", "id": "ara", "label": "Ara", "command": "ara", "table": "liste" },
            { "type": "table", "id": "liste" }
        ]
    }))
    .expect("a panel")
}

fn rows_of(app: &App) -> usize {
    app.extensions.panels[0]
        .tables
        .get("liste")
        .map_or(0, |(_, rows)| rows.len())
}

#[test]
fn a_late_answer_does_not_reach_a_panel_opened_again() {
    let mut app = crate::files_testing::app_with_drawing();
    let first = OpenPanel::new("tr.ornek.deneme", "liste", definition());
    let origin = PanelOrigin {
        ext: "tr.ornek.deneme".into(),
        panel: "liste".into(),
        button: "ara".into(),
        table: Some("liste".into()),
        instance: first.instance,
    };
    app.extensions.panels.push(first);
    app.extensions.panels[0].running = Some("ara".into());
    // Closed while the command ran, opened again.
    let _ = app.ext_panel_close(0);
    app.extensions
        .panels
        .push(OpenPanel::new("tr.ornek.deneme", "liste", definition()));
    let answer: Value = json!({ "rows": [{ "no": 1 }, { "no": 2 }], "columns": ["no"] });
    app.ext_panel_answer(&origin, Ok(answer.clone()));
    assert_eq!(
        rows_of(&app),
        0,
        "the old run's rows do not fill the new panel"
    );
    // The panel the run started from, still open, takes them.
    let mut app = crate::files_testing::app_with_drawing();
    let panel = OpenPanel::new("tr.ornek.deneme", "liste", definition());
    let origin = PanelOrigin {
        instance: panel.instance,
        ..origin
    };
    app.extensions.panels.push(panel);
    app.extensions.panels[0].running = Some("ara".into());
    app.ext_panel_answer(&origin, Ok(answer));
    assert_eq!(rows_of(&app), 2);
}

/// The code's `ctx.panel` calls of a run whose panel was closed and opened again: answered as closed, the new panel's
/// values untouched.
#[test]
fn a_late_call_of_the_code_does_not_reach_a_panel_opened_again() {
    let dir = temp("gec-cagri");
    let mut gis = app(&dir, Some(Module::Gis));
    install(&mut gis, "vitrin.kek", vitrin_package());
    let _ = gis.extensions_event(Event::OpenPanel(VITRIN.into(), "yapi-paneli".into()));
    gis.extensions.panel_calls.push(PanelCall {
        ext: VITRIN.into(),
        command: "yapi-ara".into(),
        params: json!({ "sorgu": "", "katman": null, "numarasiz": false }),
        origin: PanelOrigin {
            ext: VITRIN.into(),
            panel: "yapi-paneli".into(),
            button: "ara".into(),
            table: Some("sonuc".into()),
            instance: gis.extensions.instance_of(VITRIN, "yapi-paneli"),
        },
    });
    gis.extensions.panels[0].running = Some("ara".into());
    let package = gis
        .extensions
        .registry
        .as_ref()
        .and_then(|r| r.package_dir(VITRIN))
        .unwrap();
    let _ = gis.extensions_event(Event::Launch(
        VITRIN.into(),
        "command".into(),
        "yapi-ara".into(),
        Ok(super::super::runner::Launch::for_test(
            "/bin/sh".into(),
            package,
            "vitrin.logic:yapi_ara",
            "Yapı ara",
            false,
        )),
    ));
    // Its own panel takes the call while it is open.
    let own = gis.ext_panel_session(
        VITRIN,
        "panel.say",
        &json!({ "text": "Aranıyor…", "kind": "info" }),
    );
    assert_eq!(own.ok(), Some(json!({ "applied": true })));
    // Closed while it runs, opened again: another panel.
    let _ = gis.ext_panel_close(0);
    let _ = gis.extensions_event(Event::OpenPanel(VITRIN.into(), "yapi-paneli".into()));
    let late = gis.ext_panel_session(
        VITRIN,
        "panel.set",
        &json!({ "id": "sorgu", "value": "Y-09" }),
    );
    assert_eq!(late.ok(), Some(json!({ "applied": false, "closed": true })));
    let panel = gis.extensions.panel().expect("the panel opened again");
    assert!(
        panel.texts.get("sorgu").is_none_or(String::is_empty),
        "{:?}",
        panel.texts
    );
    assert!(panel.said.is_none());
    let _ = std::fs::remove_dir_all(&dir);
}
