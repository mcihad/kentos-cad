//! What the shown extension panel's map pickers hold, on the drawing (docs/adr/8057 §62): the area or line drawn with
//! a shape picker stays on the map while its panel is the one shown, since the rows beside it are about that area, and
//! a point picker's point is marked. Dashed in the accent on the area's halo, an area lightly filled, as the shape
//! looked while it was drawn; only the shown panel's, so two panels' areas are not confused. Nothing is written to the
//! drawing, and a component hidden (its rule, its code) shows nothing.

use iced::widget::canvas::{self, LineDash, Path, Stroke};
use iced::{Color, Point};
use kentos_extensions::ui_panel::Component;
use kentos_interaction::Vec2;
use kentos_render_wgpu::Camera;
use serde_json::Value;

use super::panels::OpenPanel;
use crate::app::App;
use crate::viewport::MarkColors;

/// A panel's picked places: each shape's points and whether it closes (an area, a rectangle, a circle), each point.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Picked {
    pub(crate) shapes: Vec<(Vec<Vec2>, bool)>,
    pub(crate) points: Vec<Vec2>,
}

impl App {
    /// The shown panel's picked places; none when it has none, or no panel is open.
    pub(crate) fn panel_marks(&self) -> Option<Picked> {
        let picked = self
            .extensions
            .panels
            .get(self.extensions.active_panel)?
            .picked();
        (!picked.shapes.is_empty() || !picked.points.is_empty()).then_some(picked)
    }
}

impl OpenPanel {
    /// Its shown pickers' places: the shapes drawn, the points given (a point typed wrong is left out).
    pub(crate) fn picked(&self) -> Picked {
        let mut out = Picked::default();
        for (at, c) in self.definition.body.iter().enumerate() {
            match c {
                Component::ShapePicker { id, .. } if self.shown(at, c) => {
                    if let Some(shape) = self.geoms.get(id).and_then(shape_points) {
                        out.shapes.push(shape);
                    }
                }
                Component::PointPicker { id, .. } if self.shown(at, c) => {
                    let typed = self.texts.get(id).map(|t| t.trim());
                    if let Some([x, y]) = typed.and_then(kentos_extensions::ui_panel::typed_point) {
                        out.points.push(Vec2::new(x, y));
                    }
                }
                _ => {}
            }
        }
        out
    }
}

/// A drawn shape's points as GeoJSON holds them (a line's, an area's outer ring) and whether it closes.
fn shape_points(shape: &Value) -> Option<(Vec<Vec2>, bool)> {
    let (coordinates, closed) = match shape["type"].as_str()? {
        "LineString" => (shape["coordinates"].as_array()?, false),
        "Polygon" => (shape["coordinates"][0].as_array()?, true),
        _ => return None,
    };
    let points: Vec<Vec2> = coordinates
        .iter()
        .filter_map(|p| Some(Vec2::new(p.get(0)?.as_f64()?, p.get(1)?.as_f64()?)))
        .collect();
    (points.len() >= 2).then_some((points, closed))
}

/// Draws the picked places: a shape dashed in the accent on a halo, an area lightly filled; a point's ring.
pub(crate) fn draw(
    frame: &mut canvas::Frame,
    picked: &Picked,
    camera: &Camera,
    colors: &MarkColors,
    accent: Color,
) {
    let screen = |p: &Vec2| {
        let [x, y] = camera.world_to_screen(*p);
        Point::new(x as f32, y as f32)
    };
    let dashed = Stroke {
        line_dash: LineDash {
            segments: &[6.0, 4.0],
            offset: 0,
        },
        ..Stroke::default().with_color(accent).with_width(1.5)
    };
    for (points, closed) in &picked.shapes {
        let path = Path::new(|b| {
            for (k, p) in points.iter().enumerate() {
                if k == 0 {
                    b.move_to(screen(p));
                } else {
                    b.line_to(screen(p));
                }
            }
            if *closed {
                b.close();
            }
        });
        if *closed {
            frame.fill(&path, Color { a: 0.1, ..accent });
        }
        frame.stroke(
            &path,
            Stroke::default().with_color(colors.halo).with_width(3.5),
        );
        frame.stroke(&path, dashed);
    }
    for p in &picked.points {
        let ring = Path::circle(screen(p), 5.0);
        frame.stroke(
            &ring,
            Stroke::default().with_color(colors.halo).with_width(4.0),
        );
        frame.stroke(&ring, Stroke::default().with_color(accent).with_width(2.0));
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// The shown panel's area and point are marked; a line stays open; a shape of no kind, a point typed wrong and
    /// a hidden picker mark nothing.
    #[test]
    fn the_shown_panels_area_and_point_are_marked() {
        use kentos_extensions::ui_panel;
        let value = json!({ "format": ui_panel::FORMAT, "schemaVersion": ui_panel::VERSION, "title": "Alan",
            "body": [
                { "type": "shapePicker", "id": "alan", "label": "Alan", "shape": "polygon" },
                { "type": "shapePicker", "id": "yol", "label": "Yol", "shape": "line" },
                { "type": "pointPicker", "id": "nokta", "label": "Nokta" },
                { "type": "pointPicker", "id": "gizli", "label": "Gizli" }
            ] });
        let mut app = crate::files_testing::app_with_drawing();
        assert_eq!(app.panel_marks(), None, "no panel");
        let mut panel = OpenPanel::new(
            "tr.ornek.deneme",
            "alan",
            ui_panel::read(&value).expect("a panel"),
        );
        panel.geoms.insert(
            "alan".into(),
            json!({ "type": "Polygon", "coordinates": [[[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [0.0, 0.0]]] }),
        );
        panel.geoms.insert(
            "yol".into(),
            json!({ "type": "LineString", "coordinates": [[0.0, 0.0], [3.0, 4.0]] }),
        );
        panel.texts.insert("nokta".into(), "10, 20".into());
        panel.texts.insert("gizli".into(), "30, 40".into());
        panel.hidden.insert("gizli".into());
        app.extensions.panels.push(panel);
        let picked = app.panel_marks().expect("its marks");
        assert_eq!(picked.shapes.len(), 2);
        assert!(picked.shapes[0].1, "an area closes");
        assert!(!picked.shapes[1].1, "a line does not");
        assert_eq!(
            picked.points,
            [Vec2::new(10.0, 20.0)],
            "the hidden one is not marked"
        );
        let panel = &mut app.extensions.panels[0];
        panel.geoms.insert(
            "alan".into(),
            json!({ "type": "Point", "coordinates": [1.0, 2.0] }),
        );
        panel.geoms.remove("yol");
        panel.texts.insert("nokta".into(), "on, yirmi".into());
        assert_eq!(app.panel_marks(), None, "nothing to mark");
    }
}
