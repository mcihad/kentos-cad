//! Active tool defaults, separate from the selected object's inspector.
use crate::app::{App, Message};
use iced::widget::{column, container};
use iced::{Element, Fill, Task};
use kentos_ui::label;
use kentos_ui::widget::property_grid::{self, PropertySheet};
use kentos_ui::widget::{EditCell, Menu};

#[derive(Clone, Debug)]
pub struct Commit {
    pub tool: &'static str,
    pub key: &'static str,
    pub value: String,
}

impl App {
    pub(crate) fn architecture_settings_view(&self) -> Element<'_, Message> {
        let rows = self.session.settings(&self.format());
        if rows.is_empty() {
            return iced::widget::space().height(0).into();
        }
        let tool = self.session.tool_id();
        let mut sheet = PropertySheet::new();
        for setting in rows {
            let key = setting.key;
            let cell: Element<'_, Message> = if !setting.editable {
                property_grid::value(format!("{} (sabit)", setting.value), false, None)
            } else if setting.choices.is_empty() {
                EditCell::new(setting.value, move |value| {
                    Message::ToolSetting(Commit { tool, key, value })
                })
                .into()
            } else {
                let choices = setting.choices;
                property_grid::choice(setting.value, None, move || {
                    let mut menu = Menu::new();
                    for c in &choices {
                        menu = if let Some(command) = c.command {
                            menu.item(c.label.clone(), Message::Run(command))
                        } else {
                            menu.item(
                                c.label.clone(),
                                Message::ToolSetting(Commit {
                                    tool,
                                    key,
                                    value: c.typed.clone(),
                                }),
                            )
                        };
                    }
                    menu
                })
            };
            sheet = sheet.row(setting.label, cell);
        }
        column![
            container(
                column![
                    label::strong("Yeni nesnenin araç ayarları"),
                    label::caption("Enter veya alandan çıkış uygular")
                ]
                .spacing(4)
            )
            .padding([6, 8]),
            sheet,
            container(label::strong("Seçili nesnenin özellikleri")).padding([10, 12]),
        ]
        .width(Fill)
        .into()
    }
    pub(crate) fn tool_setting(&mut self, event: Commit) -> Task<Message> {
        if self.session.tool_id() != event.tool || !self.session.is_running() {
            return Task::none();
        }
        if !self
            .session
            .settings(&self.format())
            .iter()
            .any(|s| s.key == event.key && s.editable)
        {
            return Task::none();
        }
        // The tool says why an invalid commit is refused. Do not replace its specific explanation.
        self.with_tool(|s, cx| s.configure(event.key, &event.value, cx));
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files_testing::{app_with_drawing, make_cad};
    #[test]
    fn defaults_are_separate_from_the_selection_and_stale_commits_are_ignored() {
        let mut app = app_with_drawing();
        make_cad(&mut app);
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let _ = app.start_tool("wallRectangle");
        let _ = app.tool_setting(Commit {
            tool: "wallRectangle",
            key: "E",
            value: "250mm".into(),
        });
        assert_eq!(app.memory.wall_width, 0.25);
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
        let _ = app.start_tool("door");
        let _ = app.tool_setting(Commit {
            tool: "wallRectangle",
            key: "E",
            value: "5m".into(),
        });
        assert_eq!(app.memory.wall_width, 0.25);
        let _ = app.tool_setting(Commit {
            tool: "door",
            key: "E",
            value: "900mm".into(),
        });
        assert!(
            app.session
                .settings(&app.format())
                .iter()
                .any(|r| r.key == "E" && r.value.contains("0.900"))
        );
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
    }
    #[test]
    fn starting_an_architectural_tool_reveals_its_settings_but_regular_line_does_not() {
        let mut app = app_with_drawing();
        make_cad(&mut app);
        app.docks.close(crate::app::Panel::Properties);
        let _ = app.start_tool("line");
        assert!(!app.docks.is_shown(crate::app::Panel::Properties));
        let _ = app.start_tool("space");
        assert!(app.docks.is_shown(crate::app::Panel::Properties));
    }

    /// Complements native-window interaction; these are explicitly offscreen
    /// GPU renders at the exact acceptance sizes, not native screenshots.
    #[test]
    #[ignore = "GPU acceptance matrix; run serially and inspect the PNGs"]
    fn precision_opening_gpu_matrix_and_raw_pointer_pan() {
        use iced::{Point, Size};
        use kentos_ui::snapshot::{Input, Snapshot};
        use kentos_ui::theme::typography::{self, Typography};
        use serde_json::json;
        let _guard = crate::appearance::tests::TYPOGRAPHY.lock();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let out = root.join(".run/architecture-openings/gpu");
        std::fs::create_dir_all(&out).unwrap();
        let update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        for mode in ["dark", "light"] {
            for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
                for font in [13, 18] {
                    let mut app = app_with_drawing();
                    let drawing = crate::document::Document::read(
                        &root.join("docs/baseline/2026-10-25-architecture-openings/after.kcad"),
                    )
                    .unwrap();
                    let _ = app.update(Message::Opened(Some(Ok(Box::new(drawing)))));
                    let _ = app.update(Message::DialogClosed);
                    app.window_size = Size::new(width, height);
                    assert!(
                        app.settings
                            .choose(&[
                                ("appearance.theme", json!(mode)),
                                ("appearance.textSize", json!(font)),
                            ])
                            .is_empty()
                    );
                    app.apply_settings();
                    app.tab = "arch";
                    for panel in [
                        crate::app::Panel::Layers,
                        crate::app::Panel::Processing,
                        crate::app::Panel::Blocks,
                        crate::app::Panel::Templates,
                        crate::app::Panel::Sources,
                    ] {
                        app.docks.close(panel);
                    }
                    app.docks.dock(
                        crate::app::Panel::Properties,
                        kentos_ui::widget::docking::Side::Right,
                    );
                    app.docks
                        .set_size(kentos_ui::widget::docking::Side::Right, 208.0);
                    let mut shot = Snapshot::new(app.window_size).unwrap();
                    assert!(
                        shot.renderer_name().contains("wgpu"),
                        "GPU required, got {}",
                        shot.renderer_name()
                    );
                    shot.settle(&mut app, App::view, &mut { update });
                    let _ = app.update(Message::Run("view.zoomExtents"));
                    shot.settle(&mut app, App::view, &mut { update });
                    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
                    let _ = app.update(Message::Run("tool.pan"));
                    shot.settle(&mut app, App::view, &mut { update });
                    let b = app.viewport.bounds;
                    let p = Point::new(b.x + b.width * 0.4, b.y + b.height * 0.5);
                    let q = Point::new(p.x + 80.0, p.y + 40.0);
                    let old = app.viewport.camera;
                    for input in [Input::Press(p), Input::Move(q), Input::Release(q)] {
                        shot.input(&mut app, App::view, &mut { update }, input);
                    }
                    let moved = app.viewport.camera;
                    assert_ne!(
                        old.world_to_screen(kentos_interaction::Vec2::new(0.0, 0.0)),
                        moved.world_to_screen(kentos_interaction::Vec2::new(0.0, 0.0)),
                        "widget pan {mode}/{width}/{font}"
                    );
                    assert_eq!(
                        app.document.as_ref().unwrap().model.to_snapshot_v2(),
                        before
                    );
                    let _ = app.update(Message::Run("tool.cancel"));
                    let _ = app.update(Message::Run("view.zoomExtents"));
                    let _ = app.update(Message::Run("tool.door"));
                    for (key, value) in [("E", "1 m"), ("U", "2 m"), ("F", "end"), ("R", "hinge")] {
                        let _ = app.tool_setting(Commit {
                            tool: "door",
                            key,
                            value: value.into(),
                        });
                    }
                    shot.settle(&mut app, App::view, &mut { update });
                    for label in ["Yöntem", "Duvar ucu", "Ölçü noktası"] {
                        let r =
                            crate::files_testing::find_text(&mut shot, &app, label).expect(label);
                        assert!(
                            r.x >= 0.0 && r.x + r.width <= width,
                            "{mode}/{width}/{font}: {label} {r:?}"
                        );
                    }
                    shot.render(app.view(), &app.theme())
                        .save(out.join(format!("door-{mode}-{width}x{height}-{font}.png")))
                        .unwrap();
                    let r =
                        crate::files_testing::find_text(&mut shot, &app, "Ölçü noktası").unwrap();
                    if r.y + r.height > height - 70.0 {
                        shot.input(
                            &mut app,
                            App::view,
                            &mut { update },
                            Input::Scroll(Point::new(width - 80.0, height * 0.65), -4.0),
                        );
                        shot.render(app.view(), &app.theme())
                            .save(
                                out.join(format!("door-lower-{mode}-{width}x{height}-{font}.png")),
                            )
                            .unwrap();
                    }
                    let _ = app.update(Message::Run("tool.window"));
                    let _ = app.tool_setting(Commit {
                        tool: "window",
                        key: "B",
                        value: "two".into(),
                    });
                    shot.settle(&mut app, App::view, &mut { update });
                    shot.input(
                        &mut app,
                        App::view,
                        &mut { update },
                        Input::Scroll(Point::new(width - 80.0, height * 0.65), 50.0),
                    );
                    assert!(
                        !app.session
                            .settings(&app.format())
                            .iter()
                            .any(|r| matches!(r.key, "E" | "U" | "F" | "R"))
                    );
                    shot.render(app.view(), &app.theme())
                        .save(out.join(format!("two-jamb-{mode}-{width}x{height}-{font}.png")))
                        .unwrap();
                    assert_eq!(
                        app.document.as_ref().unwrap().model.to_snapshot_v2(),
                        before
                    );
                }
            }
        }
        typography::set(Typography::DEFAULT);
    }
    /// Delivers a whole native-style batch with its final Iced cursor. The
    /// ordinary Snapshot::input helper deliberately delivers per event, so
    /// it cannot by itself catch this runtime ordering regression.
    #[test]
    #[ignore = "GPU native event-batch regression; run serially"]
    fn a_batched_pan_moves_out_and_back_without_editing_the_drawing() {
        use iced::{Event, Point, Size, mouse};
        use kentos_ui::snapshot::{Input, Snapshot};
        let _guard = crate::appearance::tests::TYPOGRAPHY.lock();
        let mut app = app_with_drawing();
        app.window_size = Size::new(1100.0, 650.0);
        let mut shot = Snapshot::new(app.window_size).unwrap();
        assert!(shot.renderer_name().contains("wgpu"));
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        shot.settle(&mut app, App::view, &mut update);
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let b = app.viewport.bounds;
        let p = Point::new(b.x + b.width * 0.4, b.y + b.height * 0.5);
        let q = Point::new(p.x + 80.0, p.y + 40.0);
        let _ = app.update(Message::Run("tool.pan"));
        for (from, to) in [(p, q), (q, p)] {
            // Establish the runtime's final cursor without changing the camera.
            shot.input(&mut app, App::view, &mut update, Input::Move(to));
            let old = app.viewport.camera;
            let events = [
                Event::Mouse(mouse::Event::CursorMoved { position: from }),
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::CursorMoved { position: to }),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ];
            let messages = shot.update(app.view(), &events);
            for message in messages {
                update(&mut app, message);
            }
            let a = old.world_to_screen(kentos_interaction::Vec2::new(0.0, 0.0));
            let z = app
                .viewport
                .camera
                .world_to_screen(kentos_interaction::Vec2::new(0.0, 0.0));
            assert!((z[0] - a[0] - f64::from(to.x - from.x)).abs() < 1e-9);
            assert!((z[1] - a[1] - f64::from(to.y - from.y)).abs() < 1e-9);
            assert_eq!(
                app.document.as_ref().unwrap().model.to_snapshot_v2(),
                before
            );
        }
    }

    /// The GUI's 4.772 m versus 5 m observation has a geometric source:
    /// the midpoint of the visible host face between two opening jambs.
    #[test]
    fn an_opening_grip_uses_the_visible_host_face_midpoint_when_snapping_is_on() {
        use crate::viewport::Event;
        use iced::{Point, Rectangle, Size};
        use kentos_commands::Entity;
        use kentos_interaction::{SnapKind, Vec2};
        let mut app = app_with_drawing();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let doc = crate::document::Document::read(
            &root.join("docs/baseline/2026-10-25-architecture-pointer-batches/created.kcad"),
        )
        .unwrap();
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        let _ = app.update(Message::DialogClosed);
        let _ = app.update(Message::Viewport(Event::Resized(Rectangle::new(
            Point::new(0.0, 145.0),
            Size::new(1128.0, 587.0),
        ))));
        let model = &app.document.as_ref().unwrap().model;
        let slot = model
            .slot_of("01a17033-e581-7148-baca-de2677b6dd13".parse().unwrap())
            .unwrap();
        let Entity::Opening(o) = model.get(slot).unwrap() else {
            panic!("opening")
        };
        let at = o.opening.at;
        let width = o.opening.width;
        // Independently from the visible face: previous jamb 3.5 + 1.1/2,
        // next jamb at - width/2. The midpoint is a valid snap, not the cursor.
        let expected = ((3.5 + 1.1 / 2.0) + (at - width / 2.0)) / 2.0;
        let before = model.to_snapshot_v2();
        app.selection.set([slot]);
        let screen = |app: &App, x, y| {
            let p = app.viewport.camera.world_to_screen(Vec2::new(x, y));
            Point::new(p[0] as f32, p[1] as f32)
        };
        let start = screen(&app, at, 0.0);
        for event in [
            Event::Moved(start),
            Event::Pressed(start),
            Event::Released(start),
        ] {
            let _ = app.update(Message::Viewport(event));
        }
        assert!(app.session.grip_active());
        let target = screen(&app, 5.0, 0.008064516);
        let snapped = app.pointer_at(target);
        let hit = snapped.snap.expect("host face midpoint");
        assert_eq!((hit.kind, hit.id), (SnapKind::Midpoint, 1.0));
        assert!((hit.point.x - expected).abs() < 1e-14);
        assert_eq!(hit.point.y, 0.1);
        let p = app.viewport.camera.world_to_screen(hit.point);
        let distance = (p[0] - f64::from(target.x)).hypot(p[1] - f64::from(target.y));
        assert!(distance <= app.draft.snap_aperture);
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
        let _ = app.update(Message::Viewport(Event::Moved(target)));
        assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(), before);
        let _ = app.update(Message::Viewport(Event::Pressed(target)));
        let Entity::Opening(o) = app.document.as_ref().unwrap().model.get(slot).unwrap() else {
            panic!("opening")
        };
        assert!((o.opening.at - expected).abs() < 1e-14);
        assert_eq!(o.opening.width, width);
        assert!(app.document.as_mut().unwrap().model.undo().is_some());
        assert_eq!(
            app.document
                .as_ref()
                .unwrap()
                .model
                .to_snapshot_v2()
                .entities,
            before.entities
        );
        // The same pointer without object snapping is the raw 5 m point.
        app.draft.snap = false;
        let start = screen(&app, at, 0.0);
        for event in [
            Event::Moved(start),
            Event::Pressed(start),
            Event::Released(start),
        ] {
            let _ = app.update(Message::Viewport(event));
        }
        assert!(app.session.grip_active());
        let raw = app.pointer_at(target);
        assert!(raw.snap.is_none());
        assert_eq!(raw.world.x, 5.0);
    }

}
