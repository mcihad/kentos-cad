//! Narrow architectural workspaces keep the drawing scale and every drafting aid reachable.
use crate::app::{App, Message};
use crate::catalog::{Standing, catalog};
use kentos_ui::widget::Menu;

impl App {
    /// The narrow status bar's Yardımcılar: the aids the wide one shows as lamps, the program's own
    /// (Topoloji and Çakışma are a map's, docs/adr/0802).
    pub(crate) fn compact_aids_menu(&self) -> Menu<Message> {
        let mut menu = Menu::new().header("Çizim yardımcıları");
        for (id, name) in crate::view::STATUS_AIDS {
            if !self.shows_aid(id) {
                continue;
            }
            let Some(command) = catalog().get(id) else {
                continue;
            };
            let on = if id == "draft.ortho" {
                self.ortho_shown()
            } else {
                self.checked(id).unwrap_or(false)
            };
            menu = menu
                .check(
                    name,
                    on,
                    (command.standing == Standing::Ported).then_some(Message::RunKept(id)),
                )
                .icon(command.icon)
                .stay();
            if let Some(keys) = command.shortcuts.first() {
                menu = menu.shortcut(*keys);
            }
        }
        menu = menu
            .separator()
            .submenu("Kenet seçenekleri", self.snap_cell_menu());
        if self.shows_aid("draft.overlap") {
            menu = menu.submenu("Çakışma seçenekleri", self.overlap_menu());
        }
        menu = menu.submenu("Seçim süzgeci türleri", self.select_filter_menu());
        if self.shows_aid("draft.topology") {
            menu = menu
                .check(
                    "Topolojik düzenlemede noktalar da",
                    self.checked("draft.topologyPoints").unwrap_or(false),
                    Message::RunKept("draft.topologyPoints"),
                )
                .stay();
        }
        menu
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Panel;
    use crate::files_testing::{app_with_drawing, make_cad};
    use kentos_ui::theme::typography::{self, Typography};
    use kentos_ui::widget::ribbon::{self, Group};

    #[test]
    fn architectural_tabs_fit_small_and_large_text_without_losing_named_access() {
        let _guard = crate::appearance::tests::TYPOGRAPHY.lock();
        typography::load();
        let mut app = app_with_drawing();
        make_cad(&mut app);
        let names: Vec<_> = catalog()
            .tabs_in(kentos_commands::Workspace::Cad)
            .find(|t| t.id == "arch")
            .unwrap()
            .panels
            .iter()
            .map(|p| p.label)
            .collect();
        assert_eq!(names, ["Çiz", "Yerleştir", "Düzenle", "Belgele", "Proje"]);
        for size in [13, 18] {
            let _ = app
                .settings
                .choose(&[("appearance.textSize", serde_json::json!(size))]);
            app.apply_settings();
            for id in ["arch", "ctx.wall", "ctx.opening", "ctx.space", "ctx.arch"] {
                let tab = catalog()
                    .tabs_in(kentos_commands::Workspace::Cad)
                    .chain(catalog().contextual_in(kentos_commands::Workspace::Cad))
                    .find(|t| t.id == id)
                    .unwrap();
                let groups: Vec<_> = tab
                    .panels
                    .iter()
                    .filter_map(|p| app.ribbon_group(id, p))
                    .collect();
                let widths: Vec<_> = groups.iter().map(Group::widths).collect();
                let keep: Vec<_> = groups.iter().map(Group::keeps).collect();
                for width in [1100., 1440.] {
                    let (levels, overflow) = ribbon::fit(&widths, &keep, width);
                    assert!(!overflow, "{id} {size}px/{width}px: {levels:?}");
                    assert!(
                        widths
                            .iter()
                            .zip(&levels)
                            .map(|(w, l)| w[usize::from(*l)])
                            .sum::<f32>()
                            <= width + 0.5
                    );
                }
            }
        }
        typography::set(Typography::DEFAULT);
    }

    #[test]
    fn opening_properties_twice_preserves_selection_document_and_running_draft() {
        let mut app = app_with_drawing();
        make_cad(&mut app);
        let _ = app.update(Message::Run("tool.wallRectangle"));
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let selected = app.selection.ids().to_vec();
        let prompt = app.session.prompt().text();
        app.docks.close(Panel::Properties);
        for _ in 0..2 {
            let _ = app.update(Message::Run("view.properties"));
            assert!(app.docks.is_shown(Panel::Properties));
            assert_eq!(app.selection.ids(), selected);
            assert_eq!(app.session.prompt().text(), prompt);
            assert_eq!(
                app.document.as_ref().unwrap().model.to_snapshot_v2(),
                before
            );
        }
    }

    #[test]
    fn compact_aids_keep_all_switches_and_their_detailed_options() {
        let mut app = app_with_drawing();
        make_cad(&mut app);
        let text = format!("{:?}", app.compact_aids_menu());
        for (_, label) in crate::view::STATUS_AIDS {
            assert!(text.contains(label), "{label}: {text}");
        }
        for label in [
            "Kenet seçenekleri",
            "Çakışma seçenekleri",
            "Seçim süzgeci türleri",
            "Topolojik düzenlemede noktalar da",
        ] {
            assert!(text.contains(label), "{label}: {text}");
        }
        let _ = app.update(Message::Run("tool.wallRectangle"));
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let on = app.checked("draft.grid").unwrap();
        let _ = app.update(Message::RunKept("draft.grid"));
        assert_eq!(app.checked("draft.grid"), Some(!on));
        assert_eq!(app.session.tool_id(), "wallRectangle");
        assert!(app.session.is_running());
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
    }

    #[test]
    fn kentos_mimari_s_aids_have_no_map_s_lamps_or_their_options() {
        let mut app = app_with_drawing();
        make_cad(&mut app);
        app.program = Some(kentos_launch::Module::Architecture);
        let text = format!("{:?}", app.compact_aids_menu());
        for label in [
            "Topoloji",
            "Çakışma",
            "Çakışma seçenekleri",
            "Topolojik düzenlemede noktalar da",
        ] {
            assert!(!text.contains(&format!("\"{label}\"")), "{label}: {text}");
        }
        for label in [
            "Kenet",
            "Orto",
            "Kenet seçenekleri",
            "Seçim süzgeci türleri",
        ] {
            assert!(text.contains(label), "{label}: {text}");
        }
        // A map's program has them.
        app.program = Some(kentos_launch::Module::Map);
        let text = format!("{:?}", app.compact_aids_menu());
        assert!(text.contains("Çakışma seçenekleri"), "{text}");
    }

    #[test]
    fn a_folded_architecture_group_opens_and_runs_its_named_tool() {
        use crate::catalog::Item;
        use kentos_ui::snapshot::{Input, Snapshot};
        let _guard = crate::appearance::tests::TYPOGRAPHY.lock();
        let mut app = app_with_drawing();
        make_cad(&mut app);
        let _ = app
            .settings
            .choose(&[("appearance.textSize", serde_json::json!(18))]);
        app.apply_settings();
        app.tab = "arch";
        app.window_size = iced::Size::new(800., 650.);
        let tab = catalog()
            .tabs_in(kentos_commands::Workspace::Cad)
            .find(|t| t.id == "arch")
            .unwrap();
        let groups: Vec<_> = tab
            .panels
            .iter()
            .map(|p| app.ribbon_group("arch", p).unwrap())
            .collect();
        let (levels, _) = ribbon::fit(
            &groups.iter().map(Group::widths).collect::<Vec<_>>(),
            &vec![false; groups.len()],
            800.,
        );
        let (panel, command) = tab
            .panels
            .iter()
            .zip(levels)
            .find_map(|(p, l)| {
                if l < 3 {
                    return None;
                }
                p.items.iter().find_map(|i| match i {
                    Item::Command { id, .. } if id.starts_with("tool.") => {
                        Some((p.label, catalog().get(id).unwrap()))
                    }
                    _ => None,
                })
            })
            .expect("a narrow ribbon folds at least one group containing tools");
        drop(groups);
        let mut shot = Snapshot::software(app.window_size).unwrap();
        crate::tools_screens::open_folded(&mut shot, &mut app, panel);
        let target = crate::files_testing::find_text(&mut shot, &app, command.title)
            .expect("named tool is in the folded menu");
        shot.input(
            &mut app,
            App::view,
            &mut |app, message| {
                let _ = app.update(message);
            },
            Input::Click(target.center()),
        );
        assert!(app.session.is_running());
        assert_eq!(
            Some(app.session.tool_id()),
            command.id.strip_prefix("tool.")
        );
        typography::set(Typography::DEFAULT);
    }

    #[test]
    fn narrow_large_text_status_keeps_scale_visible_and_toggles_without_ending_the_tool() {
        use kentos_ui::snapshot::{Input, Snapshot};
        let _guard = crate::appearance::tests::TYPOGRAPHY.lock();
        let mut app = app_with_drawing();
        make_cad(&mut app);
        let _ = app
            .settings
            .choose(&[("appearance.textSize", serde_json::json!(18))]);
        app.apply_settings();
        app.set_plot_scale(100.);
        app.say(
            kentos_interaction::Level::Success,
            "Mahal-Z-02 Mutfak oluşturuldu; çizim korunuyor.",
        );
        let _ = app.update(Message::Run("tool.wallRectangle"));
        app.selection.set([kentos_domain::Slot(1)]);
        app.viewport.cursor = Some(kentos_render_wgpu::Vec2::new(487009., 4419999.));
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        app.window_size = iced::Size::new(1100., 650.);
        let mut shot = Snapshot::software(app.window_size).unwrap();
        shot.settle(&mut app, App::view, &mut |app, message| {
            let _ = app.update(message);
        });
        let scale = crate::files_testing::find_texts(&mut shot, &app, "1:100")
            .into_iter()
            .find(|r| r.y >= 600. && r.y + r.height <= 650.)
            .expect("drawing scale stays in the visible status bar");
        assert!(
            scale.x >= 0. && scale.x + scale.width <= 1100. && scale.y + scale.height <= 650.,
            "{scale:?}"
        );
        let aid = crate::files_testing::find_text(&mut shot, &app, "Yardımcılar").unwrap();
        assert!(
            aid.width >= typography::measured_width("Yardımcılar", typography::body(), false),
            "compact aid name is not clipped: {aid:?}"
        );
        crate::tools_screens::press_caption(&mut shot, &mut app, "Yardımcılar", false, false);
        let target = crate::files_testing::find_text(&mut shot, &app, "Orto")
            .expect("the named aid is in the menu");
        let old = app.checked("draft.ortho");
        shot.input(
            &mut app,
            App::view,
            &mut |app, message| {
                let _ = app.update(message);
            },
            Input::Click(target.center()),
        );
        assert_eq!(app.checked("draft.ortho"), old.map(|v| !v));
        assert_eq!(app.session.tool_id(), "wallRectangle");
        assert!(app.session.is_running());
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
        typography::set(Typography::DEFAULT);
    }

    #[test]
    fn selected_architectural_context_names_stay_inside_the_narrow_tab_strip() {
        use iced::advanced::widget::{Id, Operation, operation::Scrollable};
        use kentos_ui::snapshot::Snapshot;
        use std::sync::{Arc, Mutex};
        // Ordinary text queries report content coordinates. Include the real scroll translation
        // and clipping viewport, so this assertion measures the name actually painted on screen.
        struct HeaderName {
            name: String,
            viewport: iced::Rectangle,
            translation: iced::Vector,
            found: Arc<Mutex<Option<(iced::Rectangle, iced::Rectangle)>>>,
        }
        impl Operation for HeaderName {
            fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
                operate(self);
            }
            fn scrollable(
                &mut self,
                _: Option<&Id>,
                bounds: iced::Rectangle,
                _: iced::Rectangle,
                translation: iced::Vector,
                _: &mut dyn Scrollable,
            ) {
                if bounds.y < 40. {
                    self.viewport = bounds;
                    self.translation = translation;
                }
            }
            fn text(&mut self, _: Option<&Id>, bounds: iced::Rectangle, text: &str) {
                if text == self.name && bounds.y < 40. {
                    let visible = bounds - self.translation;
                    *self.found.lock().unwrap() = Some((visible, self.viewport));
                }
            }
        }
        struct ScrollHeaderLeft;
        impl Operation for ScrollHeaderLeft {
            fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
                operate(self);
            }
            fn scrollable(
                &mut self,
                _: Option<&Id>,
                bounds: iced::Rectangle,
                _: iced::Rectangle,
                _: iced::Vector,
                state: &mut dyn Scrollable,
            ) {
                if bounds.y < 40. {
                    state.snap_to(
                        iced::advanced::widget::operation::scrollable::RelativeOffset {
                            x: Some(0.),
                            y: None,
                        },
                    );
                }
            }
        }
        let _guard = crate::appearance::tests::TYPOGRAPHY.lock();
        for (kind, slot) in [("Duvar", 1), ("Açıklık", 6), ("Mahal", 12)] {
            for width in [1100., 1440.] {
                let mut app = app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.textSize", serde_json::json!(18))]);
                app.apply_settings();
                app.window_size = iced::Size::new(width, 650.);
                let mut shot = Snapshot::software(app.window_size).unwrap();
                shot.settle(&mut app, App::view, &mut |app, message| {
                    let _ = app.update(message);
                });
                if kind == "Mahal" {
                    crate::space_scenes::spaces(&mut app);
                } else {
                    crate::opening_scenes::room(&mut app, 100.);
                }
                let expected = match kind {
                    "Duvar" => "wall",
                    "Açıklık" => "opening",
                    _ => "space",
                };
                assert_eq!(
                    app.document
                        .as_ref()
                        .unwrap()
                        .model
                        .get(kentos_domain::Slot(slot))
                        .map(|e| e.kind()),
                    Some(expected)
                );
                app.selection.set([kentos_domain::Slot(slot)]);
                app.ribbon_context = true;
                shot.settle(&mut app, App::view, &mut |app, message| {
                    let _ = app.update(message);
                });
                let found = Arc::new(Mutex::new(None));
                shot.operate(
                    app.view(),
                    Box::new(HeaderName {
                        name: kind.into(),
                        viewport: iced::Rectangle::with_size(app.window_size),
                        translation: iced::Vector::ZERO,
                        found: found.clone(),
                    }),
                );
                let (at, viewport) = found
                    .lock()
                    .unwrap()
                    .expect("context tab has its name above the tools");
                assert!(
                    at.x >= viewport.x && at.x + at.width <= viewport.x + viewport.width,
                    "{kind}/{width}: {at:?}, viewport {viewport:?}"
                );
                // A context is inserted where Mimari stood. Its index alone is not a tab key:
                // after manual scrolling, clearing selection must reveal the Mimari name again.
                shot.operate(app.view(), Box::new(ScrollHeaderLeft));
                shot.settle(&mut app, App::view, &mut |app, message| {
                    let _ = app.update(message);
                });
                let moved = Arc::new(Mutex::new(None));
                shot.operate(
                    app.view(),
                    Box::new(HeaderName {
                        name: kind.into(),
                        viewport: iced::Rectangle::with_size(app.window_size),
                        translation: iced::Vector::ZERO,
                        found: moved.clone(),
                    }),
                );
                if width == 1100. {
                    let (at, viewport) = moved.lock().unwrap().expect("manually scrolled context");
                    assert!(
                        at.x + at.width > viewport.x + viewport.width,
                        "manual scroll really moved {kind} out of view: {at:?}/{viewport:?}"
                    );
                }
                app.selection.clear();
                app.ribbon_context = false;
                app.tab = "arch";
                shot.settle(&mut app, App::view, &mut |app, message| {
                    let _ = app.update(message);
                });
                let found = Arc::new(Mutex::new(None));
                shot.operate(
                    app.view(),
                    Box::new(HeaderName {
                        name: "Mimari".into(),
                        viewport: iced::Rectangle::with_size(app.window_size),
                        translation: iced::Vector::ZERO,
                        found: found.clone(),
                    }),
                );
                let (at, viewport) = found.lock().unwrap().expect("main architecture tab");
                assert!(
                    at.x >= viewport.x && at.x + at.width <= viewport.x + viewport.width,
                    "return from {kind}/{width}: {at:?}, viewport {viewport:?}"
                );
            }
        }
        typography::set(Typography::DEFAULT);
    }
}
