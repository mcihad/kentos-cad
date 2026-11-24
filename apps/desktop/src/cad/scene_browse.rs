//! Keyboard traversal of the model's selectable objects, in drawing order.

use iced::keyboard::key::Named;
use iced::widget::{container, opaque, row};
use iced::{Center, Element, Fill, Padding, Task};
use kentos_commands::{Entity, SelectRegion};
use kentos_domain::Slot;
use kentos_interaction::{Vec2, ViewChange, selectable};
use kentos_ui::{label, style};

use crate::app::{App, Message};
use crate::keys::KeyPress;

pub(crate) const COMMAND: &str = "edit.browseScene";
const REVEAL_MARGIN: f64 = 48.0;

#[derive(PartialEq, Eq)]
struct Scope {
    document: u64,
    generation: u64,
    filters: u64,
    levels: u64,
    kinds: Option<u32>,
}

pub(crate) struct Browse {
    scope: Scope,
    slots: Vec<Slot>,
    index: usize,
    selection_version: u64,
}

impl App {
    pub(crate) fn can_browse_scene(&self) -> bool {
        self.document.is_some()
            && !self.sheets.is_active()
            && !self.window_over()
            && self.extensions.dialog.is_none()
            && self.paragraph.is_none()
            && self.text_field.is_none()
            && self.field.is_none()
            && !self.session.is_running()
            && !self.session.grip_active()
            && self.session.select_box().is_none()
    }

    fn browse_scope(&self) -> Option<Scope> {
        if !self.can_browse_scene() {
            return None;
        }
        let doc = self.document.as_ref()?;
        Some(Scope {
            document: doc.session,
            generation: doc.model.generation(),
            filters: self.spatial.filters_version(),
            levels: self.spatial.level_looks().0,
            kinds: self.draft.select_kinds,
        })
    }

    /// Enter with the selected object where possible, otherwise with the drawing's first candidate.
    pub(crate) fn toggle_scene_browse(&mut self) -> Task<Message> {
        if self.scene_browse.take().is_some() {
            return Task::none();
        }
        if !self.can_browse_scene() {
            self.output(
                "Nesnelerde gezinmek için açık model görünümüne dönün ve etkin komutu bitirin.",
            );
            return Task::none();
        }
        if !self.open_scene_browse() {
            self.output("Görünen katmanlarda seçim kurallarına uyan nesne yok.");
            return Task::none();
        }
        self.line_focused = false;
        self.layers_keyboard = false;
        self.blocks_panel.keyboard = false;
        self.template_work.templates_panel.keyboard = false;
        self.vertices.keyboard = false;
        crate::input::release_keyboard()
    }

    fn open_scene_browse(&mut self) -> bool {
        let Some(found) = self.query_selection(SelectRegion::All) else {
            return false;
        };
        let Some(scope) = self.browse_scope() else {
            return false;
        };
        if found.slots.is_empty() {
            return false;
        }
        let index = self
            .selection
            .ids()
            .last()
            .and_then(|current| found.slots.iter().position(|slot| slot == current))
            .unwrap_or(0);
        self.scene_browse = Some(Browse {
            scope,
            slots: found.slots,
            index,
            selection_version: self.selection.version(),
        });
        self.select_scene_browse_target();
        true
    }

    pub(crate) fn scene_browse_key(&mut self, press: &KeyPress) -> bool {
        if self.scene_browse.is_none()
            || press.modifiers.control()
            || press.modifiers.logo()
            || press.modifiers.alt()
        {
            return false;
        }
        let backwards = match press.named() {
            Some(Named::Tab) => press.modifiers.shift(),
            Some(Named::ArrowLeft) => true,
            Some(Named::ArrowRight) => false,
            Some(Named::Escape | Named::Enter) => {
                self.scene_browse = None;
                return true;
            }
            _ => return false,
        };
        self.step_scene_browse(backwards);
        true
    }

    fn step_scene_browse(&mut self, backwards: bool) {
        let Some(browse) = &self.scene_browse else {
            return;
        };
        let (index, target) = next_target(browse, backwards);
        // Time and other session filters can change what is visible without editing the document.
        let index = if self.scene_browse_pickable(target) {
            index
        } else {
            self.scene_browse = None;
            if !self.open_scene_browse() {
                self.output("Görünen katmanlarda seçim kurallarına uyan nesne yok.");
                return;
            }
            let Some(browse) = &self.scene_browse else {
                return;
            };
            next_target(browse, backwards).0
        };
        if let Some(browse) = self.scene_browse.as_mut() {
            browse.index = index;
        }
        self.select_scene_browse_target();
    }

    fn scene_browse_pickable(&self, slot: Slot) -> bool {
        let store = self.spatial.store();
        self.document.as_ref().is_some_and(|doc| {
            doc.model.get(slot).is_some_and(|entity| {
                selectable::on_selected_layer(&doc.model, entity)
                    && selectable::kind_allowed(self.draft.select_kinds, entity.kind())
                    && store
                        .get(f64::from(slot.0))
                        .is_some_and(|item| store.pickable(item))
            })
        })
    }

    /// Clicks, editing, an incoming selection or a changed set of layers leave keyboard traversal.
    pub(crate) fn follow_scene_browse(&mut self) {
        if let Some(browse) = &self.scene_browse
            && (self.browse_scope().as_ref() != Some(&browse.scope)
                || self.line_focused
                || self.selection.version() != browse.selection_version
                || !self.scene_browse_pickable(browse.slots[browse.index]))
        {
            self.scene_browse = None;
        }
    }

    fn select_scene_browse_target(&mut self) {
        let Some(browse) = self.scene_browse.as_mut() else {
            return;
        };
        let target = browse.slots[browse.index];
        self.selection.set([target]);
        self.selection.set_hover(None);
        browse.selection_version = self.selection.version();

        let Some(bounds) = self.spatial.store().extent(Some(&[f64::from(target.0)])) else {
            return;
        };
        let camera = &self.viewport.camera;
        // Use screen-space bounds so a turned view and a map projection keep their own orientation.
        let mut low = [f64::INFINITY; 2];
        let mut high = [f64::NEG_INFINITY; 2];
        for x in [bounds.min_x, bounds.max_x] {
            for y in [bounds.min_y, bounds.max_y] {
                let point = camera.world_to_screen(Vec2::new(x, y));
                if !point.iter().all(|value| value.is_finite()) {
                    return;
                }
                for axis in 0..2 {
                    low[axis] = low[axis].min(point[axis]);
                    high[axis] = high[axis].max(point[axis]);
                }
            }
        }
        if camera.width > 0.0
            && camera.height > 0.0
            && (low[0] < 0.0 || low[1] < 0.0 || high[0] > camera.width || high[1] > camera.height)
        {
            let padding = REVEAL_MARGIN.min(camera.width.min(camera.height) * 0.15);
            let change = if high[0] - low[0] > camera.width - 2.0 * padding
                || high[1] - low[1] > camera.height - 2.0 * padding
            {
                ViewChange::Fit { bounds, padding }
            } else {
                ViewChange::Pan {
                    dx: camera.width / 2.0 - f64::midpoint(low[0], high[0]),
                    dy: camera.height / 2.0 - f64::midpoint(low[1], high[1]),
                }
            };
            self.navigating(|app| app.viewport.change(change));
        }
    }

    pub(crate) fn scene_browse_strip(&self) -> Option<Element<'_, Message>> {
        let browse = self.scene_browse.as_ref()?;
        let doc = &self.document.as_ref()?.model;
        let entity = doc.get(browse.slots[browse.index])?;
        let (kind, name) = match entity {
            Entity::Opening(opening) => (
                opening.opening.form.label(),
                opening.opening.number.as_deref(),
            ),
            Entity::Space(space) => (
                "Mahal",
                space
                    .space
                    .name
                    .as_deref()
                    .or(space.space.number.as_deref()),
            ),
            entity => (
                crate::selecting::kind_title(entity.kind()),
                entity.base().label.as_deref(),
            ),
        };
        let title = name.filter(|name| !name.trim().is_empty()).map_or_else(
            || format!("{kind} #{}", entity.base().id),
            |name| format!("{kind} {name}"),
        );
        let layer = doc
            .layers()
            .get(&entity.base().layer_id)
            .map(|layer| layer.name.as_str());
        let mut summary = match layer {
            Some(layer) => format!("{title} · {layer}"),
            None => title,
        };
        if doc.layers().is_locked(&entity.base().layer_id) {
            summary.push_str(" · Kilitli");
        }
        let strip = container(
            row![
                label::strong(format!(
                    "Nesne {} / {}",
                    browse.index + 1,
                    browse.slots.len()
                )),
                label::body(summary),
                label::caption("Tab / Shift+Tab gezin  ·  Enter / Esc bitir"),
            ]
            .spacing(12.0)
            .align_y(Center)
            .wrap()
            .vertical_spacing(4.0),
        )
        .padding([7, 12])
        .max_width((self.viewport.camera.width as f32 - 32.0).clamp(240.0, 720.0))
        .style(style::container::popover);
        Some(
            container(opaque(strip))
                .width(Fill)
                .align_x(Center)
                .padding(Padding {
                    top: 10.0,
                    ..Padding::ZERO
                })
                .into(),
        )
    }
}

fn next_target(browse: &Browse, backwards: bool) -> (usize, Slot) {
    let n = browse.slots.len();
    let i = if backwards {
        browse.index.checked_sub(1).unwrap_or(n - 1)
    } else {
        (browse.index + 1) % n
    };
    (i, browse.slots[i])
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key::{Code, Named, Physical};
    use iced::keyboard::{Key, Modifiers};

    use super::*;
    use crate::selecting::tests::objects;

    fn key(app: &mut App, named: Named, code: Code, modifiers: Modifiers) {
        let _ = app.update(Message::Key(KeyPress {
            key: Key::Named(named),
            physical: Physical::Code(code),
            modifiers,
            text: None,
            repeat: false,
        }));
    }

    #[test]
    fn scene_browse_routes_the_shortcut_wraps_in_both_directions_and_keeps_the_selection() {
        let mut app = objects();
        let revision = app.document.as_ref().unwrap().model.revision();

        let _ = app.update(Message::Key(KeyPress {
            key: Key::Character("J".into()),
            physical: Physical::Code(Code::KeyJ),
            modifiers: Modifiers::CTRL | Modifiers::SHIFT,
            text: None,
            repeat: false,
        }));
        assert_eq!(app.selection.ids(), &[Slot(1)]);
        assert!(app.scene_browse_strip().is_some());

        key(&mut app, Named::Tab, Code::Tab, Modifiers::SHIFT);
        assert_eq!(app.selection.ids(), &[Slot(6)], "hidden layer 7 is skipped");
        key(&mut app, Named::Tab, Code::Tab, Modifiers::empty());
        assert_eq!(app.selection.ids(), &[Slot(1)]);
        key(
            &mut app,
            Named::ArrowRight,
            Code::ArrowRight,
            Modifiers::empty(),
        );
        assert_eq!(app.selection.ids(), &[Slot(2)]);

        key(&mut app, Named::Escape, Code::Escape, Modifiers::empty());
        assert!(app.scene_browse_strip().is_none());
        assert_eq!(app.selection.ids(), &[Slot(2)]);
        assert_eq!(app.document.as_ref().unwrap().model.revision(), revision);
    }

    #[test]
    fn scene_browse_keeps_the_current_object_on_entry_and_respects_the_selection_filter() {
        let mut app = objects();
        app.selection.set([Slot(4)]);
        let _ = app.update(Message::Run(COMMAND));
        assert_eq!(app.selection.ids(), &[Slot(4)]);
        key(&mut app, Named::Tab, Code::Tab, Modifiers::empty());
        assert_eq!(app.selection.ids(), &[Slot(5)]);
        key(&mut app, Named::Enter, Code::Enter, Modifiers::empty());
        assert_eq!(app.selection.ids(), &[Slot(5)]);
        app.draft.select_kinds = Some(selectable::bit("point"));
        let _ = app.update(Message::Run(COMMAND));
        assert_eq!(app.scene_browse.as_ref().unwrap().slots, [Slot(5)]);
        key(&mut app, Named::Tab, Code::Tab, Modifiers::empty());
        assert_eq!(app.selection.ids(), &[Slot(5)]);

        let _ = app.update(Message::Run("tool.line"));
        assert!(app.scene_browse.is_none());
        assert!(app.session.is_running(), "a drawing tool can take over");
        key(&mut app, Named::Tab, Code::Tab, Modifiers::empty());
        assert_eq!(app.selection.ids(), &[Slot(5)]);
    }

    #[test]
    fn scene_browse_brings_an_offscreen_entity_into_view_without_changing_scale() {
        let mut app = objects();
        app.viewport
            .camera
            .center_on(Vec2::new(485000.0, 4425000.0));
        let scale = app.viewport.camera.scale;
        let _ = app.update(Message::Run(COMMAND));

        assert_eq!(app.selection.ids(), &[Slot(1)]);
        assert_eq!(app.viewport.camera.scale, scale);
        let [x, y] = app
            .viewport
            .camera
            .world_to_screen(Vec2::new(486984.0, 4419988.0));
        assert!(x > 0.0 && x < app.viewport.camera.width, "{x}");
        assert!(y > 0.0 && y < app.viewport.camera.height, "{y}");
    }

    #[test]
    fn scene_browse_fits_an_offscreen_object_larger_than_the_view() {
        let mut app = objects();
        app.viewport
            .camera
            .center_on(Vec2::new(485000.0, 4425000.0));
        app.viewport.camera.scale = 1000.0;

        let _ = app.update(Message::Run(COMMAND));

        assert_eq!(app.selection.ids(), &[Slot(1)]);
        assert!(app.viewport.camera.scale < 1000.0);
        for x in [486976.0, 486992.0] {
            let [x, y] = app.viewport.camera.world_to_screen(Vec2::new(x, 4419988.0));
            assert!(x > 0.0 && x < app.viewport.camera.width, "{x}");
            assert!(y > 0.0 && y < app.viewport.camera.height, "{y}");
        }
    }

    #[test]
    fn scene_browse_reveals_a_line_when_the_camera_is_inside_its_bounds_but_away_from_its_stroke() {
        let mut app = objects();
        app.selection.set([Slot(2)]);
        app.viewport
            .camera
            .center_on(Vec2::new(487020.0, 4420005.0));
        app.viewport.camera.scale = 1000.0;

        let _ = app.update(Message::Run(COMMAND));

        assert_eq!(app.selection.ids(), &[Slot(2)]);
        assert!(app.viewport.camera.scale < 1000.0);
        let [x, y] = app
            .viewport
            .camera
            .world_to_screen(Vec2::new(487000.0, 4420004.0));
        assert!(x > 0.0 && x < app.viewport.camera.width, "{x}");
        assert!(y > 0.0 && y < app.viewport.camera.height, "{y}");
    }
}
