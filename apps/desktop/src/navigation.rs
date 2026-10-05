//! Gezinme (docs/adr/0141): the view history behind Önceki görünüm and
//! Sonraki görünüm, and Kapsam denetimi.
//!
//! - **The history** is `kentos_interaction::ViewHistory`, kept by the app as
//!   session state: it never enters the drawing, undo or a file, and opening
//!   another drawing empties it (`App::show_document`). A view is recorded as
//!   the user leaves it, by:
//!   - the navigation commands: Tümünü göster, Seçime yakınlaştır, Katmana
//!     yakınlaştır, Yakınlaştır and Uzaklaştır ([`App::navigating`]), and
//!     Pencere yakınlaştır, whose box the tool asks the camera for
//!     (`ViewChange::Fit`, input.rs);
//!   - the start of a pan: the middle button's first step ([`App::keep_view`])
//!     or the Kaydır tool's (input.rs);
//!   - the wheel's first step after it rested for 500 ms: a run of steps is one
//!     pass ([`App::keep_view`]).
//!
//!   A window resizing records nothing. A navigation that leaves the view
//!   where it was (Tümünü göster on the extents, a zoom at its limit) records
//!   nothing either, so that Önceki görünüm always moves.
//! - **Kapsam denetimi** selects the visible objects far from the rest of the
//!   drawing, through the store's `extent_outliers`, and says how many. It
//!   moves and deletes nothing: what to do with them is the user's.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use iced::Task;
use kentos_interaction::{Level, Viewpoint};

use crate::app::{App, Message};
use crate::viewport;

/// The web command ids this module runs.
pub const COMMANDS: [&str; 3] = ["view.previous", "view.next", "view.extentCheck"];

/// The monotonic time the wheel's passes are told by: since the first ask.
fn clock() -> Duration {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed()
}

impl App {
    pub(crate) fn navigation_command(&mut self, id: &'static str) -> Task<Message> {
        match id {
            "view.previous" => self.view_back(),
            "view.next" => self.view_forward(),
            "view.extentCheck" => self.extent_check(),
            _ => {}
        }
        Task::none()
    }

    /// Where the camera stands.
    pub(crate) fn viewpoint(&self) -> Viewpoint {
        self.viewport.viewpoint()
    }

    /// Runs a navigation: a change of the view whose old view the history
    /// keeps, when the change moved it (docs/adr/0141).
    pub(crate) fn navigating<T>(&mut self, navigate: impl FnOnce(&mut Self) -> T) -> T {
        let before = self.viewpoint();
        let out = navigate(self);
        if self.viewpoint() != before {
            self.view_history.record(before);
        }
        out
    }

    /// What the drawing area reported, for the history: the middle button's
    /// first step of a pan and the wheel's first step after a rest record the
    /// view `before` the event moved it. `was_panning`: the pan was already
    /// going on, so its start is past.
    pub(crate) fn keep_view(
        &mut self,
        event: &viewport::Event,
        before: Viewpoint,
        was_panning: bool,
    ) {
        if self.viewpoint() == before {
            return;
        }
        match event {
            viewport::Event::Panned { .. } if !was_panning => self.view_history.record(before),
            viewport::Event::Extents => self.view_history.record(before),
            viewport::Event::Zoomed { .. } => {
                let at = self.view_clock.unwrap_or_else(clock);
                self.view_history.record_wheel(before, at);
            }
            _ => {}
        }
    }

    /// Önceki görünüm: back to the last view left, this one kept for Sonraki görünüm.
    fn view_back(&mut self) {
        let here = self.viewpoint();
        if let Some(view) = self.view_history.back(here) {
            self.viewport.restore(view);
        }
    }

    /// Sonraki görünüm: the reverse.
    fn view_forward(&mut self) {
        let here = self.viewpoint();
        if let Some(view) = self.view_history.forward(here) {
            self.viewport.restore(view);
        }
    }

    /// Kapsam denetimi: the far objects become the selection; with none, the
    /// selection is left as it was.
    fn extent_check(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        self.spatial.sync(&doc.model);
        let far = self.spatial.extent_outliers();
        if far.is_empty() {
            self.say(Level::Info, "Çizimin kapsamını bozan nesne yok.");
            return;
        }
        let n = far.len();
        self.selection.set(far);
        self.say(
            Level::Warn,
            format!("{n} nesne çizimin geri kalanından çok uzakta; seçildi."),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use iced::{Point, Rectangle, Size, Vector};
    use kentos_contracts::{Entity, EntityBase, PointEntity, Vec2};
    use kentos_domain::Slot;
    use kentos_interaction::Level;

    use crate::app::{App, Message};
    use crate::files_testing::{app_with_drawing, drawing, last_said};
    use crate::layering::Event as LayerEvent;
    use crate::viewport::Event;

    fn run(app: &mut App, id: &'static str) {
        let _ = app.update(Message::Run(id));
    }

    fn viewport(app: &mut App, event: Event) {
        let _ = app.update(Message::Viewport(event));
    }

    /// The sample on an area 1000 × 800 logical pixels, fitted to its start view.
    fn open() -> App {
        let mut app = app_with_drawing();
        viewport(
            &mut app,
            Event::Resized(Rectangle::new(Point::ORIGIN, Size::new(1000.0, 800.0))),
        );
        assert!(!app.view_history.can_back(), "opening records nothing");
        app
    }

    /// One turn of the wheel at `ms` on the wheel's own clock.
    fn wheel(app: &mut App, ms: u64) {
        app.view_clock = Some(Duration::from_millis(ms));
        viewport(
            app,
            Event::Zoomed {
                factor: 1.05,
                at: Point::new(500.0, 400.0),
            },
        );
    }

    fn click(app: &mut App, at: Point) {
        viewport(app, Event::Moved(at));
        viewport(app, Event::Pressed(at));
        viewport(app, Event::Released(at));
    }

    #[test]
    fn the_history_commands_are_off_until_there_is_a_view_to_go_to() {
        let mut app = open();
        assert!(!app.available("view.previous") && !app.available("view.next"));
        run(&mut app, "view.zoomIn");
        assert!(app.available("view.previous") && !app.available("view.next"));
        run(&mut app, "view.previous");
        assert!(!app.available("view.previous") && app.available("view.next"));
        run(&mut app, "view.next");
        assert!(app.available("view.previous") && !app.available("view.next"));
    }

    /// The commands go between the views left, and say nothing.
    #[test]
    fn previous_and_next_go_between_the_views_left() {
        let mut app = open();
        let (v0, said) = (app.viewpoint(), app.log.len());
        run(&mut app, "view.zoomIn");
        let v1 = app.viewpoint();
        run(&mut app, "view.zoomIn");
        let v2 = app.viewpoint();
        assert!(v0 != v1 && v1 != v2);
        run(&mut app, "view.previous");
        assert_eq!(app.viewpoint(), v1);
        run(&mut app, "view.previous");
        assert_eq!(app.viewpoint(), v0);
        // No more: the command changes nothing.
        run(&mut app, "view.previous");
        assert_eq!(app.viewpoint(), v0);
        run(&mut app, "view.next");
        assert_eq!(app.viewpoint(), v1);
        run(&mut app, "view.next");
        assert_eq!(app.viewpoint(), v2);
        run(&mut app, "view.next");
        assert_eq!(app.viewpoint(), v2);
        assert_eq!(app.log.len(), said, "no messages");
    }

    #[test]
    fn a_new_navigation_deletes_the_next_views() {
        let mut app = open();
        run(&mut app, "view.zoomIn");
        run(&mut app, "view.zoomIn");
        run(&mut app, "view.previous");
        assert!(app.available("view.next"));
        run(&mut app, "view.zoomOut");
        assert!(!app.available("view.next"));
        // The view left, before the zoom out, is one to go back to.
        run(&mut app, "view.previous");
        run(&mut app, "view.previous");
        assert!(!app.available("view.previous"));
    }

    /// Every navigation command records the view it leaves, except that
    /// which leaves it where it was, and a window resizing records nothing.
    #[test]
    fn the_navigation_commands_record_and_a_resize_does_not() {
        let mut app = open();
        let mut recorded = 0;
        let mut check = |app: &mut App, id: &'static str, moves: bool| {
            let before = app.viewpoint();
            run(app, id);
            assert_eq!(app.viewpoint() != before, moves, "{id}");
            recorded += usize::from(moves);
            assert_eq!(app.view_history.len(), recorded, "{id}");
        };
        check(&mut app, "view.zoomExtents", true);
        // Tümünü göster again: the view is where it was, so nothing is recorded.
        check(&mut app, "view.zoomExtents", false);
        check(&mut app, "view.zoomOut", true);
        // Seçime yakınlaştır with nothing selected: nothing happens.
        check(&mut app, "view.zoomSelection", false);
        app.selection.set([Slot(4)]);
        check(&mut app, "view.zoomSelection", true);
        check(&mut app, "view.zoomIn", true);
        viewport(
            &mut app,
            Event::Resized(Rectangle::new(Point::ORIGIN, Size::new(900.0, 600.0))),
        );
        assert_eq!(
            app.view_history.len(),
            recorded,
            "a resize is no navigation"
        );
    }

    #[test]
    fn a_middle_double_click_shows_everything_and_is_recorded() {
        let mut app = open();
        let before = app.viewpoint();
        viewport(&mut app, Event::Extents);
        assert_ne!(app.viewpoint(), before);
        assert_eq!(app.view_history.len(), 1);
        run(&mut app, "view.previous");
        assert_eq!(app.viewpoint(), before);
    }

    /// The wheel is one pass: only its first step, after 500 ms of rest, records.
    #[test]
    fn the_wheel_is_one_pass_until_it_rests() {
        let mut app = open();
        let start = app.viewpoint();
        for ms in [1_000, 1_100, 1_200, 1_300] {
            wheel(&mut app, ms);
        }
        assert_eq!(app.view_history.len(), 1, "one pass");
        let zoomed = app.viewpoint();
        wheel(&mut app, 1_700);
        assert_eq!(
            app.view_history.len(),
            1,
            "400 ms after the last step: the same pass"
        );
        wheel(&mut app, 2_200);
        assert_eq!(app.view_history.len(), 2, "500 ms of rest starts another");
        run(&mut app, "view.previous");
        assert!(app.viewpoint() != start);
        run(&mut app, "view.previous");
        assert_eq!(app.viewpoint(), start, "the pass left the view it began at");
        run(&mut app, "view.next");
        assert!(app.viewpoint() != zoomed);
    }

    /// The middle button records where a pan starts, once per drag.
    #[test]
    fn a_middle_button_pan_records_where_it_starts() {
        let mut app = open();
        let start = app.viewpoint();
        let pan = |app: &mut App, x: f32| {
            viewport(
                app,
                Event::Panned {
                    by: Vector::new(x, 3.0),
                    at: Point::new(100.0 + x, 100.0),
                },
            );
        };
        for x in [10.0, 20.0, 30.0] {
            pan(&mut app, x);
        }
        assert_eq!(app.view_history.len(), 1);
        // The next plain move ends the pan; the next pan is another one.
        viewport(&mut app, Event::Moved(Point::new(300.0, 300.0)));
        pan(&mut app, 12.0);
        pan(&mut app, 12.0);
        assert_eq!(app.view_history.len(), 2);
        run(&mut app, "view.previous");
        run(&mut app, "view.previous");
        assert_eq!(app.viewpoint(), start);
    }

    /// Kaydır records the start of each drag; a click that moves nothing records nothing.
    #[test]
    fn the_pan_tool_records_the_start_of_each_drag() {
        let mut app = open();
        let start = app.viewpoint();
        run(&mut app, "tool.pan");
        let drag = |app: &mut App, by: f32| {
            let from = Point::new(300.0, 300.0);
            viewport(app, Event::Moved(from));
            viewport(app, Event::Pressed(from));
            for k in 1..=3 {
                viewport(app, Event::Moved(Point::new(300.0 + by * k as f32, 300.0)));
            }
            viewport(app, Event::Released(Point::new(300.0 + by * 3.0, 300.0)));
        };
        click(&mut app, Point::new(400.0, 400.0));
        assert_eq!(app.view_history.len(), 0, "a click moves nothing");
        drag(&mut app, 20.0);
        assert_eq!(app.view_history.len(), 1);
        assert!(app.viewpoint() != start);
        drag(&mut app, -8.0);
        assert_eq!(app.view_history.len(), 2, "each drag");
        run(&mut app, "view.previous");
        run(&mut app, "view.previous");
        assert_eq!(app.viewpoint(), start);
    }

    #[test]
    fn a_zoom_window_is_a_navigation() {
        let mut app = open();
        let start = app.viewpoint();
        run(&mut app, "tool.zoomWindow");
        click(&mut app, Point::new(300.0, 250.0));
        click(&mut app, Point::new(600.0, 450.0));
        assert!(!app.session.is_running(), "the tool leaves");
        assert_ne!(app.viewpoint(), start);
        assert_eq!(app.view_history.len(), 1);
        run(&mut app, "view.previous");
        assert_eq!(app.viewpoint(), start);
    }

    #[test]
    fn opening_another_drawing_empties_the_history() {
        let mut app = open();
        run(&mut app, "view.zoomIn");
        run(&mut app, "view.zoomIn");
        run(&mut app, "view.previous");
        assert!(app.available("view.previous") && app.available("view.next"));
        let _ = app.update(Message::Opened(Some(Ok(Box::new(drawing(3))))));
        assert!(!app.available("view.previous") && !app.available("view.next"));
    }

    /// The ADR's case, through the wheel: 31 navigations keep 30 views; 30
    /// steps back reach the second, 30 forward come home.
    #[test]
    fn thirty_one_navigations_go_back_thirty_steps_and_forward_again() {
        let mut app = open();
        let mut cameras = vec![app.viewpoint()];
        for pass in 0..31 {
            wheel(&mut app, 1_000 + pass * 1_000);
            cameras.push(app.viewpoint());
        }
        assert_eq!(app.view_history.len(), 30);
        for step in 1..=30 {
            assert!(app.available("view.previous"), "back {step}");
            run(&mut app, "view.previous");
            assert_eq!(app.viewpoint(), cameras[31 - step], "back {step}");
        }
        assert!(
            !app.available("view.previous"),
            "the first view was dropped"
        );
        for step in 1..=30 {
            assert!(app.available("view.next"), "forward {step}");
            run(&mut app, "view.next");
            assert_eq!(app.viewpoint(), cameras[1 + step], "forward {step}");
        }
        assert!(!app.available("view.next"));
        // A new navigation now deletes what is ahead (nothing), and is recorded.
        run(&mut app, "view.zoomOut");
        assert!(app.available("view.previous"));
    }

    /// A point on `layer`, at the given place (metres east and north).
    fn add_point(app: &mut App, layer: &str, x: f64, y: f64) -> Slot {
        let doc = app.document.as_mut().expect("open");
        doc.model
            .add(Entity::Point(PointEntity {
                base: EntityBase {
                    id: 0,
                    layer_id: layer.to_owned(),
                    color: None,
                    attrs: Default::default(),
                    label: None,
                    symbol: None,
                    line_weight: None,
                },
                p: Vec2 { x, y },
                z: None,
                parts: None,
            }))
            .expect("a slot")
    }

    #[test]
    fn extent_check_selects_the_objects_far_from_the_drawing() {
        let mut app = open();
        assert!(app.available("view.extentCheck"));
        // The sample holds four visible objects: nothing lies far from them.
        app.selection.set([Slot(4)]);
        let before = app.selection.ids().to_vec();
        run(&mut app, "view.extentCheck");
        assert_eq!(last_said(&app), "Çizimin kapsamını bozan nesne yok.");
        assert_eq!(app.last_level, Some(Level::Info));
        assert_eq!(
            app.selection.ids(),
            before,
            "the selection is left as it was"
        );
        // A point that fell to zero is far; the drawing did not move.
        let far = add_point(&mut app, "parsel", 0.0, 0.0);
        let revision = app.document.as_ref().expect("open").model.revision();
        run(&mut app, "view.extentCheck");
        assert_eq!(
            last_said(&app),
            "1 nesne çizimin geri kalanından çok uzakta; seçildi."
        );
        assert_eq!(app.last_level, Some(Level::Warn));
        assert_eq!(
            app.selection.ids(),
            [far],
            "the far object is the selection"
        );
        let doc = app.document.as_ref().expect("open");
        assert_eq!(
            doc.model.revision(),
            revision,
            "nothing is moved or deleted"
        );
        assert!(doc.model.get(far).is_some());
        // The Çizim layer is hidden in the sample: what is not seen is not counted.
        add_point(&mut app, "cizim", 10.0, 10.0);
        run(&mut app, "view.extentCheck");
        assert_eq!(app.selection.ids(), [far], "hidden objects are not far");
        // Two far objects: the count. Up to a quarter of the objects may be far and
        // still be found, so the drawing needs more near ones first.
        for i in 0..4 {
            add_point(&mut app, "parsel", 486_520.0 + f64::from(i), 4_420_200.0);
        }
        let second = add_point(&mut app, "parsel", 5.0, -40_000.0);
        run(&mut app, "view.extentCheck");
        assert_eq!(
            last_said(&app),
            "2 nesne çizimin geri kalanından çok uzakta; seçildi."
        );
        assert_eq!(app.selection.ids(), [far, second]);
        assert!(!app.view_history.can_back(), "the view is not touched");
    }

    #[test]
    fn extent_check_has_no_drawing_to_check_without_one() {
        let (mut app, _) = App::boot(None);
        assert!(!app.available("view.extentCheck"));
        run(&mut app, "view.extentCheck");
        assert_eq!(last_said(&app), "Açık çizim yok.");
    }

    /// The text of a layer menu's zoom entry and whether it can be pressed.
    fn zoom_entry(app: &App, id: &str) -> (String, bool) {
        let doc = app.document.as_ref().expect("open");
        let node = doc.model.layers().get(id).expect("the layer").clone();
        let menu = format!("{:?}", app.layer_menu(&node, false));
        let from = menu.find("yakınlaştır").expect("the entry");
        let start = menu[..from].rfind("label: \"").expect("its label") + "label: \"".len();
        let label = menu[start..from + "yakınlaştır".len()].to_owned();
        let pressable = menu[from..]
            .split("on_press: ")
            .nth(1)
            .is_some_and(|rest| rest.starts_with("Some(Layer(ZoomTo("));
        (label, pressable)
    }

    #[test]
    fn a_layers_menu_zooms_to_its_objects_and_is_recorded() {
        let mut app = open();
        assert_eq!(
            zoom_entry(&app, "parsel"),
            ("Katmana yakınlaştır".to_owned(), true)
        );
        assert_eq!(
            zoom_entry(&app, "layer-g"),
            ("Gruba yakınlaştır".to_owned(), true)
        );
        let start = app.viewpoint();
        let _ = app.update(Message::Layer(LayerEvent::ZoomTo("parsel".into())));
        // The view is centred on the parcel's box (its edges' arcs included).
        let parcel = app
            .spatial
            .store()
            .extent(Some(&[4.0]))
            .expect("the parcel");
        let c = app.viewport.camera.center;
        assert!(
            (c.x - (parcel.min_x + parcel.max_x) / 2.0).abs() < 1e-9
                && (c.y - (parcel.min_y + parcel.max_y) / 2.0).abs() < 1e-9,
            "{c:?} {parcel:?}"
        );
        let zoomed = app.viewpoint();
        assert_ne!(zoomed, start);
        assert!(app.available("view.previous"), "it goes into the history");
        // A group: every layer below it.
        let _ = app.update(Message::Layer(LayerEvent::ZoomTo("layer-g".into())));
        assert_eq!(app.view_history.len(), 2);
        run(&mut app, "view.previous");
        assert_eq!(app.viewpoint(), zoomed);
        run(&mut app, "view.previous");
        assert_eq!(app.viewpoint(), start);
    }

    #[test]
    fn a_layer_without_objects_cannot_be_zoomed_to() {
        let mut app = open();
        // The new layer is empty; the group that holds it and nothing else too.
        app.new_layer();
        let empty = app
            .document
            .as_ref()
            .expect("open")
            .model
            .layers()
            .active()
            .to_owned();
        assert_eq!(
            zoom_entry(&app, &empty),
            ("Katmana yakınlaştır".to_owned(), false)
        );
        let before = app.viewpoint();
        let _ = app.update(Message::Layer(LayerEvent::ZoomTo(empty)));
        assert_eq!(app.viewpoint(), before);
        assert!(!app.view_history.can_back());
    }
}
