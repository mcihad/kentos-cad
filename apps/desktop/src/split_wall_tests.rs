//! Each view of a split drawing area draws the walls at its own camera's detail (docs/adr/0696's 29 October
//! addendum): the active one's is the store's, which it picks and snaps at; the others draw theirs from the store's
//! joined walls; a press that makes another view active hands each the detail it drew, so neither changes.

use iced::{Point, Rectangle, Size};
use kentos_geometry_core::geom::wall::{Detail, View};
use kentos_native_style::batches::{BatchKind, FillPaintBatch};
use serde_json::json;

use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, make_cad};
use crate::tools_scenes::{Objects, open};
use crate::viewport::{Event, Viewport};

/// Two walls of plaster, brick and plaster, the brick hatched, joined in an L, in a CAD project at 1:100 whose walls'
/// detail follows the view; the area split in two side by side, both views on the walls.
fn split_on_walls() -> App {
    let mut app = app_with_drawing();
    let mut o = Objects::new();
    for (a, b) in [([0.0, 0.0], [6.0, 0.0]), ([6.0, 0.0], [6.0, 4.0])] {
        o.push(
            "cizim",
            json!({
                "kind": "wall", "pts": [{ "x": a[0], "y": a[1] }, { "x": b[0], "y": b[1] }],
                "width": 0.24, "justify": "center", "fill": "#5A5A5A",
                "components": [
                    { "name": "Sıva", "thickness": 0.02, "detail": "detailed" },
                    { "name": "Tuğla", "detail": "standard",
                      "hatch": { "pattern": "ANSI31", "scale": 1.0, "angle": 0.0 } },
                    { "name": "Sıva", "thickness": 0.02, "detail": "detailed" }
                ]
            }),
        );
    }
    open(&mut app, o);
    make_cad(&mut app);
    {
        let model = &mut app.document.as_mut().expect("a drawing").model;
        let mut settings = model.settings().clone();
        settings.plot_scale = 100.0;
        settings.arch = None;
        model.set_settings(settings);
    }
    let _ = app.update(Message::Viewport(Event::Resized(Rectangle::new(
        Point::ORIGIN,
        Size::new(600.0, 700.0),
    ))));
    let _ = app.update(Message::Run("view.split"));
    let _ = app.update(Message::OtherViewport(
        0,
        Event::Resized(Rectangle::new(
            Point::new(602.0, 0.0),
            Size::new(600.0, 700.0),
        )),
    ));
    app
}

fn other(app: &mut App) -> &mut Viewport {
    &mut app.split.as_mut().expect("split").others[0].viewport
}

/// What a view's styled scene fills its walls with: the body's solid fills (the schematic look) and the brick's
/// hatches (standard and detailed).
fn fills(app: &mut App, view: Option<usize>) -> (usize, usize) {
    let doc = app.document.as_ref().expect("a drawing");
    app.spatial.sync(&doc.model);
    let palette = crate::viewport::palette(app.canvas());
    let styling = app.styles.styling(&app.spatial, &app.settings);
    let viewport = match view {
        None => &app.viewport,
        Some(i) => &app.split.as_ref().expect("split").others[i].viewport,
    };
    let scene = viewport.styled_scene(doc, &palette, styling);
    let (mut solid, mut hatch) = (0, 0);
    for part in &scene.layers {
        for b in &part.layer.batches {
            match &b.kind {
                BatchKind::Fill {
                    paint: FillPaintBatch::Solid { .. },
                } => solid += 1,
                BatchKind::Fill {
                    paint: FillPaintBatch::Hatch { .. },
                } => hatch += 1,
                _ => {}
            }
        }
    }
    (solid, hatch)
}

#[test]
fn each_view_draws_the_walls_at_its_own_cameras_detail() {
    let mut app = split_on_walls();
    // The active view far out (1:200 on the screen), the other close in (1:20).
    app.viewport.camera.zoom_to_screen_scale(200.0);
    other(&mut app).camera.zoom_to_screen_scale(20.0);
    app.follow_wall_view();
    assert_eq!(app.spatial.store().wall_view().detail, Detail::Schematic);
    assert_eq!(
        other(&mut app).wall_view,
        Some(View {
            detail: Detail::Detailed,
            scale: 100.0
        }),
        "the close view's own detail, the project's paper scale for the patterns"
    );
    // The far view fills the bodies, the close one hatches the brick: each as a store at its detail draws.
    let (far_solid, far_hatch) = fills(&mut app, None);
    let (near_solid, near_hatch) = fills(&mut app, Some(0));
    assert!(far_solid > 0 && far_hatch == 0, "{far_solid} {far_hatch}");
    assert!(
        near_hatch > 0 && near_solid == 0,
        "{near_solid} {near_hatch}"
    );

    // A press on the close view makes it active: the store takes its detail, the far one keeps schematic.
    let _ = app.update(Message::OtherViewport(
        0,
        Event::Pressed(Point::new(900.0, 350.0)),
    ));
    let _ = app.update(Message::Viewport(Event::Released(Point::new(900.0, 350.0))));
    assert_eq!(app.spatial.store().wall_view().detail, Detail::Detailed);
    assert_eq!(
        app.viewport.wall_view, None,
        "the active view draws the store's"
    );
    assert_eq!(
        other(&mut app).wall_view.map(|v| v.detail),
        Some(Detail::Schematic)
    );
    let (solid, hatch) = fills(&mut app, None);
    assert!(hatch > 0 && solid == 0, "{solid} {hatch}");
    let (solid, hatch) = fills(&mut app, Some(0));
    assert!(solid > 0 && hatch == 0, "{solid} {hatch}");

    // One view again: it draws the store's.
    let _ = app.update(Message::Run("view.split"));
    assert!(app.split.is_none());
    assert_eq!(app.viewport.wall_view, None);
    assert_eq!(app.spatial.store().wall_view().detail, Detail::Detailed);
}

/// A view's own detail keeps its 10 % band (docs/adr/0696): at 1:32 a view that drew detailed goes on detailed, and
/// made the active one it brings that to the store, where the store's own schematic would have given standard.
#[test]
fn a_view_made_active_goes_on_at_the_detail_it_drew() {
    let mut app = split_on_walls();
    app.viewport.camera.zoom_to_screen_scale(200.0);
    other(&mut app).camera.zoom_to_screen_scale(20.0);
    app.follow_wall_view();
    other(&mut app).camera.zoom_to_screen_scale(32.0);
    app.follow_wall_view();
    assert_eq!(
        other(&mut app).wall_view.map(|v| v.detail),
        Some(Detail::Detailed),
        "1:32 is within the detailed band from detailed"
    );
    let _ = app.update(Message::OtherViewport(
        0,
        Event::Pressed(Point::new(900.0, 350.0)),
    ));
    let _ = app.update(Message::Viewport(Event::Released(Point::new(900.0, 350.0))));
    assert_eq!(app.spatial.store().wall_view().detail, Detail::Detailed);
    // Zoomed out past the band, it is standard.
    app.viewport.camera.zoom_to_screen_scale(40.0);
    app.follow_wall_view();
    assert_eq!(app.spatial.store().wall_view().detail, Detail::Standard);
}

/// A project's own detail (Proje ayarları › Mimari) is every view's: no view draws another.
#[test]
fn a_projects_own_detail_is_every_views() {
    let mut app = split_on_walls();
    {
        let model = &mut app.document.as_mut().expect("a drawing").model;
        let mut settings = model.settings().clone();
        settings.arch = Some(kentos_commands::ArchSettings {
            detail: Some(kentos_commands::WallDetail::Standard),
            ..Default::default()
        });
        model.set_settings(settings);
    }
    app.viewport.camera.zoom_to_screen_scale(200.0);
    other(&mut app).camera.zoom_to_screen_scale(20.0);
    app.follow_wall_view();
    let store = app.spatial.store().wall_view();
    assert_eq!(store.detail, Detail::Standard);
    assert_eq!(other(&mut app).wall_view, Some(store));
    let (_, near_hatch) = fills(&mut app, Some(0));
    let (_, far_hatch) = fills(&mut app, None);
    assert_eq!(near_hatch, far_hatch);
}

/// Kaydırmalı karşılaştır draws the other view over the active one at its scale (docs/adr/0523 §3): at the active
/// view's detail too, even where the other's own band would have kept another, so the divider never parts two.
#[test]
fn a_compared_view_draws_the_active_views_detail() {
    let mut app = split_on_walls();
    app.viewport.camera.zoom_to_screen_scale(20.0);
    other(&mut app).camera.zoom_to_screen_scale(200.0);
    app.follow_wall_view();
    assert_eq!(
        other(&mut app).wall_view.map(|v| v.detail),
        Some(Detail::Schematic)
    );
    let _ = app.update(Message::Run("view.swipe"));
    assert!(app.split.as_ref().is_some_and(|s| s.swipe.is_some()));
    // The other view now stands at the active one's scale: the active view's detail.
    let store = app.spatial.store().wall_view();
    assert_eq!(store.detail, Detail::Detailed);
    assert_eq!(other(&mut app).wall_view, Some(store));
    let (solid, hatch) = fills(&mut app, Some(0));
    assert!(hatch > 0 && solid == 0, "{solid} {hatch}");
}
