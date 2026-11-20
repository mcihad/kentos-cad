//! Alan böl's Hedef alan (H, docs/adr/1069) in a Harita CAD project: a four-cornered area, its south edge clicked, 450 m² typed; the cut
//! parallel to the edge, the piece asked for filled and both sizes shown; Enter cuts it. With `KENTOS_SHOTS=1` the
//! pictures go to `.run/gui-evidence/ur2-052/`. Test code only.

use crate::app::App;
use crate::tools_scenes::{E, N, Objects, click, open, run, typed};

/// An area of 1 152 m² whose south edge runs a little north of east.
fn ground() -> Objects {
    let mut o = Objects::new();
    o.path("parsel", &[[0.0, 0.0], [40.0, 4.0], [36.0, 36.0], [2.0, 30.0]], true);
    o
}

fn opened(app: &mut App, shot: &mut kentos_ui::snapshot::Snapshot) {
    open(app, ground());
    {
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.settings().clone();
        settings.workspace = Some(kentos_commands::Workspace::Cad);
        settings.cad_work = Some(kentos_commands::CadWork::Map);
        settings.plot_scale = 500.0;
        doc.model.set_settings(settings);
    }
    app.tab = "plan";
    app.command_expanded = false;
    // The drawing area sized first, so that the clicks land where the camera shows.
    shot.settle(app, App::view, &mut |app: &mut App, m| drop(app.update(m)));
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds { min_x: E - 8.0, min_y: N - 6.0, max_x: E + 48.0, max_y: N + 42.0 },
        24.0,
    );
}

fn shot(s: &mut kentos_ui::snapshot::Snapshot, app: &mut App, name: &str) {
    if std::env::var_os("KENTOS_SHOTS").is_none() {
        return;
    }
    s.settle(app, App::view, &mut |app: &mut App, m| drop(app.update(m)));
    crate::files_testing::evidence(s, app, "ur2-052", name);
}

#[test]
fn alan_bol_hedef_alan_cuts_the_area_parallel_to_its_edge() {
    let (mut app, _) = App::boot(None);
    let mut s = crate::files_testing::offscreen(iced::Size::new(1440., 900.));
    opened(&mut app, &mut s);
    let area = {
        let doc = &app.document.as_ref().expect("open").model;
        doc.entities_with_uids().find_map(|(uid, _)| doc.slot_of(uid)).expect("the area")
    };
    app.selection.set([area]);
    run(&mut app, "tool.areaSplit");
    assert_eq!(app.session.tool_id(), "areaSplit");
    typed(&mut app, "H");
    click(&mut app, [20.0, 2.0]);
    typed(&mut app, "450");
    shot(&mut s, &mut app, "alan-bol-hedef-alan");
    let before = app.document.as_ref().expect("open").model.len();
    let _ = app.update(crate::app::Message::CommandSubmitted);
    let doc = &app.document.as_ref().expect("open").model;
    let said = crate::files_testing::last_said(&app);
    assert_eq!(doc.len(), before + 1, "the area cut in two: {said}");
    assert!(said.contains("2 parçaya bölündü") && said.contains("450"), "{said}");
    shot(&mut s, &mut app, "alan-bol-hedef-alan-yazildi");
}
