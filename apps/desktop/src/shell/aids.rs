//! The drafting aids' cells while a command takes points (docs/adr/0268 §2): Orto shows what rules the pointer
//! now, Shift turning it over while held; Kutupsal is shown on but out of use while ortho rules (ortho comes
//! first, the two are never both).

use crate::app::App;

impl App {
    /// Whether Shift turns ortho over now: held while a command takes points.
    fn ortho_turned(&self) -> bool {
        self.modifiers.shift() && self.session.takes_points()
    }

    /// Orto's cell: lit when ortho rules the pointer now.
    pub(crate) fn ortho_shown(&self) -> bool {
        kentos_interaction::ortho_now(self.draft.ortho, self.ortho_turned())
    }

    /// Orto's note while Shift turns it over.
    pub(crate) fn ortho_note(&self) -> Option<String> {
        self.ortho_turned().then(|| {
            if self.ortho_shown() {
                "Shift basılı: geçici olarak açık.".to_owned()
            } else {
                "Shift basılı: geçici olarak kapalı.".to_owned()
            }
        })
    }

    /// Kutupsal's note while it is on and ortho rules: on, out of use.
    pub(crate) fn polar_note(&self) -> Option<String> {
        (self.draft.polar.is_some() && self.ortho_shown()).then(|| {
            "Orto açıkken kutupsal izleme uygulanmaz; Orto kapanınca kendiliğinden sürer."
                .to_owned()
        })
    }
}

impl App {
    /// What rules the next point's place with the pointer where it is (docs/adr/0268 §1), as the running command
    /// reads it: the last pointer's snap and tracking, Shift as held now; none off the drawing.
    pub(crate) fn aid_now(&mut self) -> Option<kentos_interaction::Aid> {
        let raw = self.viewport.cursor?;
        let [x, y] = self.viewport.camera.world_to_screen(raw);
        let pointer =
            kentos_interaction::Pointer::new(raw, [x, y], self.modifiers.shift(), self.snap)
                .tracked(self.tracking.track().map(|t| t.point));
        self.with_tool(|s, cx| s.ruling_aid(&pointer, cx))
    }
}

#[cfg(test)]
mod tests {
    use iced::keyboard::Modifiers;

    use crate::app::Message;
    use crate::tools_scenes::{Objects, click, hover, open, run};

    /// The polyline's preview path's last point, east and north of the scenes' origin.
    fn preview_end(app: &crate::app::App) -> [f64; 2] {
        let preview = app.session.preview(&app.format()).expect("a preview");
        let p = *preview.path.last().expect("a cursor segment");
        [p.x - crate::tools_scenes::E, p.y - crate::tools_scenes::N]
    }

    fn near(a: [f64; 2], b: [f64; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6
    }

    #[test]
    fn shift_turns_ortho_over_and_the_preview_follows_with_the_pointer_still() {
        let mut app = crate::files_testing::app_with_drawing();
        open(&mut app, Objects::new());
        let _ = app
            .settings
            .choose(&[("drafting.snap", serde_json::Value::Bool(false))]);
        app.apply_settings();
        run(&mut app, "tool.polyline");
        click(&mut app, [0.0, 0.0]);
        hover(&mut app, [20.0, 3.0]);
        let free = preview_end(&app);
        assert!(!app.ortho_shown());
        let _ = app.update(Message::Modifiers(Modifiers::SHIFT));
        assert!(
            app.ortho_shown(),
            "Orto's cell shows it on while Shift is held"
        );
        assert!(app.ortho_note().is_some());
        let held = preview_end(&app);
        assert!(near(held, [free[0], 0.0]), "{free:?} → {held:?}");
        let _ = app.update(Message::Modifiers(Modifiers::empty()));
        assert!(near(preview_end(&app), free), "let go: free again");
        assert_eq!(app.session.point_count(), 1, "the command's points stay");
    }

    #[test]
    fn polar_is_out_of_use_while_ortho_rules_and_toggling_keeps_the_command() {
        let mut app = crate::files_testing::app_with_drawing();
        open(&mut app, Objects::new());
        let _ = app
            .settings
            .choose(&[("drafting.snap", serde_json::Value::Bool(false))]);
        app.apply_settings();
        run(&mut app, "tool.polyline");
        click(&mut app, [0.0, 0.0]);
        click(&mut app, [10.0, 0.0]);
        // A whole pixel (a metre here) east of the diagonal.
        hover(&mut app, [16.0, 4.0]);
        run(&mut app, "draft.polar");
        assert_eq!(app.polar_note(), None, "ortho off: polar works");
        // On the 45° ray from (10, 0), the preview at once.
        let on = preview_end(&app);
        assert!((on[1] - (on[0] - 10.0)).abs() < 1e-6, "{on:?}");
        run(&mut app, "draft.ortho");
        assert!(app.polar_note().is_some(), "ortho rules: polar out of use");
        let square = preview_end(&app);
        assert!(square[1].abs() < 1e-6, "ortho: along the axis {square:?}");
        assert_eq!(app.session.tool_id(), "polyline");
        assert_eq!(app.session.point_count(), 2, "the points stay");
        run(&mut app, "draft.ortho");
        run(&mut app, "draft.polar");
    }
}
