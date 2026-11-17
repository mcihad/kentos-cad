//! A panel's point picker (docs/adr/8057 §57, plan/eklenti.md ADIM 22's "panelden harita seçimi"): its field takes a
//! point typed as `x, y`, or Haritadan seç picks one on the map as the processing window's point parameters do
//! (`PickPoint`: snaps apply, the command line says for which field, Esc leaves the field as it was). The point
//! picked is the field's words to the millimetre and a change: its `onChange` command runs.
//!
//! - **The panel stays:** a dock panel stands beside the map, so it is not put aside while the point is picked; the
//!   pick is the window's tool until a point or Esc.
//! - **Not in a dialog:** `ctx.dialog`'s window holds the window until it is answered, so no point is picked from it.

use iced::Task;
use iced::widget::{button, row};
use kentos_extensions::ui_panel::{Component, point_words};
use kentos_interaction::pick::PickPoint;
use kentos_interaction::{Level, Vec2};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::widget::{Tip, tip};

use super::panels::{OpenPanel, PanelEvent};
use crate::app::{App, Message};

/// A point being picked for a panel's field: whose panel and which field.
#[derive(Clone, Debug)]
pub(crate) struct PointPick {
    pub(crate) ext: String,
    pub(crate) panel: String,
    pub(crate) field: String,
}

/// A point picker's field and its Haritadan seç, as the form draws them: the processing window's pick (its target
/// icon), the words on hover. A captioned button left the field a few pixels in a narrow dock (real window, 16 November
/// 2026: "x, y" cut to its first letter in a 300-pixel dock).
pub(super) fn point_field<'a>(
    p: &'a OpenPanel,
    id: &'a str,
    usable: bool,
    event: fn(PanelEvent) -> Message,
    enter: Option<Message>,
) -> iced::Element<'a, Message> {
    let value = p.texts.get(id).map_or("", String::as_str);
    let mut field = kentos_ui::widget::text_field("x, y", value)
        .id(super::panel_session::field_id(&p.ext, &p.local, id));
    let mut pick = button(icon(Icon::Target).size(14.0).tone(Tone::Inherit))
        .padding([5, 8])
        .style(kentos_ui::style::button::secondary);
    if usable {
        let typing = id.to_owned();
        field = field.on_input(move |t| event(PanelEvent::Text(typing.clone(), t)));
        if let Some(press) = enter {
            field = field.on_submit(press);
        }
        pick = pick.on_press(event(PanelEvent::PickPoint(id.to_owned())));
    }
    let press = usable.then(|| event(PanelEvent::PickPoint(id.to_owned())));
    let pick = super::panel_view::keyed(
        tip(
            pick,
            Tip::new("Haritadan seç").body("Noktayı haritada gösterin (kenetlenir); Esc vazgeçer."),
            iced::widget::tooltip::Position::Top,
        ),
        press,
    );
    row![field, pick]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .into()
}

/// A shape picker's field (docs/adr/8057 §62): what was drawn in words, Haritada çiz and ✕.
pub(super) fn shape_field<'a>(
    p: &'a OpenPanel,
    id: &'a str,
    usable: bool,
    event: fn(PanelEvent) -> Message,
) -> iced::Element<'a, Message> {
    let drawn = p.geoms.get(id);
    let words = drawn.map_or_else(|| "Çizilmedi".to_owned(), shape_words);
    let text = if drawn.is_some() {
        kentos_ui::label::body(words)
    } else {
        kentos_ui::label::body(words).style(kentos_ui::style::text::muted)
    };
    let mut draw = button(icon(Icon::Target).size(14.0).tone(Tone::Inherit))
        .padding([5, 8])
        .style(kentos_ui::style::button::secondary);
    let mut clear = button(icon(Icon::Close).size(12.0).tone(Tone::Inherit))
        .padding([5, 8])
        .style(kentos_ui::style::button::secondary);
    let drawing = usable.then(|| event(PanelEvent::DrawShape(id.to_owned())));
    let clearing =
        (usable && drawn.is_some()).then(|| event(PanelEvent::ClearShape(id.to_owned())));
    draw = draw.on_press_maybe(drawing.clone());
    clear = clear.on_press_maybe(clearing.clone());
    let draw = super::panel_view::keyed(
        tip(
            draw,
            Tip::new("Haritada çiz").body(
                "Şekli haritada çizin (kenetlenir); Enter ya da çift tık bitirir, Esc vazgeçer.",
            ),
            iced::widget::tooltip::Position::Top,
        ),
        drawing,
    );
    let clear = super::panel_view::keyed(clear.into(), clearing);
    row![iced::widget::container(text).width(iced::Fill), draw, clear]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .into()
}

/// A drawn shape in a field's words: a line's points and length, an area's corners and size, a circle's radius and
/// size (metres, a decimal comma). The field's label names it; a number keeps its word on its line (a no-break space),
/// so a narrow dock breaks after the comma and not before "m²".
pub(crate) fn shape_words(shape: &serde_json::Value) -> String {
    let xy = |p: &serde_json::Value| -> Option<(f64, f64)> {
        Some((p.get(0)?.as_f64()?, p.get(1)?.as_f64()?))
    };
    let decimal = |v: f64| format!("{v:.1}").replace('.', ",");
    let points: Vec<(f64, f64)> = match shape["type"].as_str() {
        Some("LineString") => shape["coordinates"]
            .as_array()
            .map(|c| c.iter().filter_map(xy).collect()),
        Some("Polygon") => shape["coordinates"][0]
            .as_array()
            .map(|c| c.iter().filter_map(xy).collect()),
        _ => None,
    }
    .unwrap_or_default();
    if shape["type"] == "LineString" {
        let length: f64 = points
            .windows(2)
            .map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1))
            .sum();
        return format!("{}\u{a0}nokta, {}\u{a0}m", points.len(), decimal(length));
    }
    let area = points
        .windows(2)
        .map(|w| w[0].0 * w[1].1 - w[1].0 * w[0].1)
        .sum::<f64>()
        .abs()
        / 2.0;
    match shape["circle"]["radius"].as_f64() {
        Some(r) => format!(
            "yarıçap\u{a0}{}\u{a0}m, {}\u{a0}m²",
            decimal(r),
            decimal(area)
        ),
        None => format!(
            "{}\u{a0}köşe, {}\u{a0}m²",
            points.len().saturating_sub(1),
            decimal(area)
        ),
    }
}

impl App {
    /// Haritadan seç on panel `at`'s point picker `id`: a point is picked on the map, snaps applying; the command line
    /// names the panel and the field.
    pub(super) fn ext_panel_pick_point(&mut self, at: usize, id: &str) -> Task<Message> {
        let Some(panel) = self.extensions.panels.get(at) else {
            return Task::none();
        };
        let label = panel
            .definition
            .body
            .iter()
            .find_map(|c| match c {
                Component::PointPicker { id: own, label, .. } if own == id => Some(label.clone()),
                _ => None,
            })
            .unwrap_or_else(|| id.to_owned());
        let said = format!("{}: {label}", panel.definition.title);
        let pick = PointPick {
            ext: panel.ext.clone(),
            panel: panel.local.clone(),
            field: id.to_owned(),
        };
        self.say(Level::Command, said);
        self.extensions.point_pick = Some(pick);
        self.field = None;
        self.snap = None;
        self.session.run(Box::new(PickPoint::new("", label)));
        self.with_tool(|s, cx| s.activate(cx));
        Task::none()
    }

    /// Haritada çiz on panel `at`'s shape picker `id`: the shape its definition asks for drawn on the map with KentOS's
    /// capture (a map tool's), snaps applying; the command line names the panel and the field.
    pub(super) fn ext_panel_draw_shape(&mut self, at: usize, id: &str) -> Task<Message> {
        use kentos_extensions::ui_panel::PickShape;
        use kentos_interaction::capture::{Capture, Kind};
        let Some(panel) = self.extensions.panels.get(at) else {
            return Task::none();
        };
        let Some((label, shape)) = panel.definition.body.iter().find_map(|c| match c {
            Component::ShapePicker {
                id: own,
                label,
                shape,
                ..
            } if own == id => Some((label.clone(), *shape)),
            _ => None,
        }) else {
            return Task::none();
        };
        let kind = match shape {
            PickShape::Line => Kind::Line,
            PickShape::Polygon => Kind::Polygon,
            PickShape::Rectangle => Kind::Rectangle,
            PickShape::Circle => Kind::Circle,
        };
        let said = format!("{}: {label}", panel.definition.title);
        let pick = PointPick {
            ext: panel.ext.clone(),
            panel: panel.local.clone(),
            field: id.to_owned(),
        };
        self.say(Level::Command, said);
        self.extensions.shape_pick = Some(pick);
        self.session.run(Box::new(Capture::new(label, kind)));
        Task::none()
    }

    /// The shape drawn (or Esc) while a panel's field asked for one (`ext_capture_tasks`): the field's shape and its
    /// `onChange` command; none when no panel asked (a map tool's capture).
    pub(crate) fn ext_panel_shape_drawn(
        &mut self,
        shape: Option<kentos_interaction::capture::Shape>,
    ) -> Option<Task<Message>> {
        let pick = self.extensions.shape_pick.take()?;
        let Some(shape) = shape else {
            return Some(Task::none());
        };
        let Some(at) = self
            .extensions
            .panels
            .iter()
            .position(|x| x.ext == pick.ext && x.local == pick.panel)
        else {
            return Some(Task::none());
        };
        let panel = &mut self.extensions.panels[at];
        panel.forget_invalid(&PanelEvent::ClearShape(pick.field.clone()));
        panel
            .geoms
            .insert(pick.field.clone(), super::runner::shape_json(&shape));
        panel.recalculate();
        Some(self.ext_panel_changed(at, &pick.field))
    }

    /// A point picked (or Esc) while a panel's field asked for one: the field's words and its `onChange` command;
    /// false when no panel asked.
    pub(crate) fn ext_panel_point_picked(&mut self, p: Option<Vec2>) -> bool {
        let Some(pick) = self.extensions.point_pick.take() else {
            return false;
        };
        let Some(p) = p else {
            return true;
        };
        let Some(at) = self
            .extensions
            .panels
            .iter()
            .position(|x| x.ext == pick.ext && x.local == pick.panel)
        else {
            return true;
        };
        let words = point_words([p.x, p.y]);
        let panel = &mut self.extensions.panels[at];
        panel.forget_invalid(&PanelEvent::Picked(pick.field.clone(), words.clone()));
        panel.texts.insert(pick.field.clone(), words);
        panel.recalculate();
        // Its command after the update (`ext_point_tasks`): a view's change gives no task of its own.
        self.extensions.point_changed = Some((at, pick.field));
        true
    }

    /// After the update: the `onChange` command of the field a point was picked for (the window's update asks, as
    /// for a map tool's shape). A pick held in `later` waited for the extension's process to speak and never ran
    /// (real window, 16 November 2026: the button stayed pressed, nothing listed).
    pub(crate) fn ext_point_tasks(&mut self) -> Task<Message> {
        match self.extensions.point_changed.take() {
            Some((at, field)) => self.ext_panel_changed(at, &field),
            None => Task::none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use kentos_extensions::ui_panel;
    use kentos_interaction::Vec2;
    use serde_json::json;

    use super::super::panels::{OpenPanel, PanelEvent};

    fn panel() -> OpenPanel {
        let value = json!({ "format": ui_panel::FORMAT, "schemaVersion": ui_panel::VERSION, "title": "Yakın yapılar",
            "body": [
                { "type": "pointPicker", "id": "nokta", "label": "Nokta", "required": true, "onChange": "bul" },
                { "type": "pointPicker", "id": "ikinci", "label": "İkinci", "value": [1.5, 2.0] }
            ] });
        OpenPanel::new(
            "tr.ornek.deneme",
            "yakin",
            ui_panel::read(&value).expect("a panel"),
        )
    }

    /// A point picker's value: `[x, y]` from what was typed or given, none when empty; a required one left empty and
    /// one that is not a point are said by its label.
    #[test]
    fn a_point_pickers_value_is_x_and_y() {
        let mut p = panel();
        assert_eq!(
            p.texts["ikinci"], "1.500, 2.000",
            "its own value shown to the millimetre"
        );
        let (id, why) = p.check_params().unwrap_err();
        assert_eq!(id, "nokta");
        assert!(why.contains("nokta verilmedi"), "{why}");
        p.texts
            .insert("nokta".into(), "500123,45; 4401234,5".into());
        let values = p.check_params().expect("its values");
        assert_eq!(values["nokta"], json!([500123.45, 4401234.5]));
        assert_eq!(values["ikinci"], json!([1.5, 2.0]));
        p.texts.insert("nokta".into(), "kuzey".into());
        assert!(p.check_params().unwrap_err().1.contains("bir nokta olmalı"));
        // Its code gives it a point; not a point is the code's mistake.
        let given = super::super::panel_session::set(
            &mut p,
            &json!({ "id": "ikinci", "value": [3, 4] }),
            None,
        );
        assert_eq!(given.map_err(|e| e.message), Ok(true));
        assert_eq!(p.texts["ikinci"], "3.000, 4.000");
        let wrong = super::super::panel_session::set(
            &mut p,
            &json!({ "id": "ikinci", "value": "3, 4" }),
            None,
        );
        assert!(wrong.unwrap_err().message.contains("[x, y]"));
    }

    /// A number field's − and +: by its step from its value (or its lower bound when empty), within its bounds, to
    /// the step's decimals.
    #[test]
    fn a_number_fields_step_moves_it_within_its_bounds() {
        let value = json!({ "format": ui_panel::FORMAT, "schemaVersion": ui_panel::VERSION, "title": "Adım",
            "body": [
                { "type": "numberField", "id": "uzaklik", "label": "Uzaklık", "value": 25, "step": 5, "min": 5,
                  "max": 30 },
                { "type": "numberField", "id": "oran", "label": "Oran", "step": 0.1, "min": 0 }
            ] });
        let mut p = OpenPanel::new(
            "tr.ornek.deneme",
            "adim",
            ui_panel::read(&value).expect("a panel"),
        );
        p.step("uzaklik", 1);
        assert_eq!(p.texts["uzaklik"], "30");
        p.step("uzaklik", 1);
        assert_eq!(p.texts["uzaklik"], "30", "its upper bound");
        for _ in 0..3 {
            p.step("oran", 1);
        }
        assert_eq!(p.texts["oran"], "0,3", "no drift past the step's decimals");
        p.texts.insert("oran".into(), String::new());
        p.step("oran", -1);
        assert_eq!(p.texts["oran"], "0", "from its lower bound, not below it");
    }

    /// A shape picker (docs/adr/8057 §62): empty its value is none and a required one is said; the shape drawn is the
    /// field's GeoJSON and its words; the code sets a shape of its type, not another; ✕ lets it go.
    #[test]
    fn a_shape_drawn_on_the_map_is_the_fields() {
        use kentos_interaction::capture::Shape;
        let value = json!({ "format": ui_panel::FORMAT, "schemaVersion": ui_panel::VERSION, "title": "Alandaki yapılar",
            "body": [
                { "type": "shapePicker", "id": "alan", "label": "Alan", "shape": "polygon", "required": true }
            ] });
        let mut app = crate::files_testing::app_with_drawing();
        app.extensions.panels.push(OpenPanel::new(
            "tr.ornek.deneme",
            "alan",
            ui_panel::read(&value).expect("a panel"),
        ));
        let (id, why) = app.extensions.panels[0].check_params().unwrap_err();
        assert_eq!(id, "alan");
        assert!(why.contains("çizilmedi"), "{why}");
        let _ = app.ext_panel_draw_shape(0, "alan");
        assert!(app.extensions.shape_pick.is_some(), "drawing");
        let square = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];
        assert!(
            app.ext_panel_shape_drawn(Some(Shape::Polygon(square)))
                .is_some()
        );
        let p = &app.extensions.panels[0];
        let values = p.check_params().expect("its values");
        assert_eq!(values["alan"]["type"], "Polygon");
        assert_eq!(
            values["alan"]["coordinates"][0].as_array().map(Vec::len),
            Some(5),
            "closed"
        );
        assert_eq!(
            super::shape_words(&values["alan"]),
            "4\u{a0}köşe, 100,0\u{a0}m²"
        );
        assert!(
            app.ext_panel_shape_drawn(None).is_none(),
            "no field asks any more"
        );
        // Its code: a line is not an area; null lets it go.
        let p = &mut app.extensions.panels[0];
        let line = json!({ "type": "LineString", "coordinates": [[0.0, 0.0], [3.0, 4.0]] });
        let wrong =
            super::super::panel_session::set(p, &json!({ "id": "alan", "value": line }), None);
        assert!(wrong.unwrap_err().message.contains("Polygon"));
        assert_eq!(super::shape_words(&line), "2\u{a0}nokta, 5,0\u{a0}m");
        super::super::panel_session::set(p, &json!({ "id": "alan", "value": null }), None)
            .expect("let go");
        assert!(p.geoms.is_empty());
    }

    /// A change with an input it asks for still empty (the area let go with ✕, the box ticked before an area is
    /// drawn) starts nothing and says nothing; the rows and words given to the values before go. A button says it.
    #[test]
    fn a_change_waits_quietly_for_the_area_and_its_old_rows_go() {
        let value = json!({ "format": ui_panel::FORMAT, "schemaVersion": ui_panel::VERSION, "title": "Alandaki yapılar",
            "body": [
                { "type": "shapePicker", "id": "alan", "label": "Alan", "shape": "polygon", "required": true,
                  "onChange": "alandaki-yapilar" },
                { "type": "checkbox", "id": "tamami", "label": "Tamamı", "onChange": "alandaki-yapilar" },
                { "type": "button", "id": "bul", "label": "Bul", "command": "alandaki-yapilar" },
                { "type": "table", "id": "sonuc", "label": "Sonuç" }
            ] });
        let mut app = crate::files_testing::app_with_drawing();
        let mut panel = OpenPanel::new(
            "tr.ornek.deneme",
            "alan",
            ui_panel::read(&value).expect("a panel"),
        );
        panel.geoms.insert(
            "alan".into(),
            json!({ "type": "Polygon", "coordinates": [[[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [0.0, 0.0]]] }),
        );
        panel.tables.insert(
            "sonuc".into(),
            (vec!["no".into()], vec![json!({ "no": "1" })]),
        );
        panel.said = Some((
            crate::exchange::words::Kind::Ok,
            "6 m²'lik alanda 1 yapı.".into(),
        ));
        app.extensions.panels.push(panel);
        let _ = app.ext_panel_event(PanelEvent::ClearShape("alan".into()));
        let _ = app.ext_panel_event(PanelEvent::Check("tamami".into(), true));
        let p = &app.extensions.panels[0];
        assert!(p.geoms.is_empty());
        assert_eq!(
            (&p.said, &p.invalid, &p.running),
            (&None, &None, &None),
            "nothing said, nothing run"
        );
        assert!(!p.tables.contains_key("sonuc"), "the old area's rows went");
        assert!(app.extensions.panel_calls.is_empty());
        let _ = app.ext_panel_event(PanelEvent::Press("bul".into()));
        let p = &app.extensions.panels[0];
        assert!(
            p.said
                .as_ref()
                .is_some_and(|(_, why)| why.contains("çizilmedi")),
            "{:?}",
            p.said
        );
        assert_eq!(p.invalid.as_ref().map(|(id, _)| id.as_str()), Some("alan"));
    }

    /// Haritadan seç: the pick is the window's tool; the point picked is the field's words, Esc leaves them.
    #[test]
    fn a_point_picked_on_the_map_is_the_fields() {
        let mut app = crate::files_testing::app_with_drawing();
        app.extensions.panels.push(panel());
        let _ = app.ext_panel_pick_point(0, "nokta");
        assert!(app.extensions.point_pick.is_some(), "picking");
        assert!(app.ext_panel_point_picked(Some(Vec2::new(10.0, 20.5))));
        assert_eq!(app.extensions.panels[0].texts["nokta"], "10.000, 20.500");
        assert!(app.extensions.point_pick.is_none());
        // Its onChange runs after the update, not when the extension's process next speaks.
        assert_eq!(app.extensions.point_changed, Some((0, "nokta".to_owned())));
        let _ = app.ext_point_tasks();
        assert!(app.extensions.point_changed.is_none());
        let panel = &app.extensions.panels[0];
        assert!(
            panel.running.is_some() || panel.said.is_some(),
            "its command was asked (here no extension is installed to run it: said on the panel)"
        );
        let _ = app.ext_panel_pick_point(0, "nokta");
        assert!(app.ext_panel_point_picked(None), "Esc");
        assert_eq!(app.extensions.panels[0].texts["nokta"], "10.000, 20.500");
        assert!(
            !app.ext_panel_point_picked(Some(Vec2::new(1.0, 1.0))),
            "no field asked"
        );
    }
}
