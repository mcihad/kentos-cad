//! A data layer's selected rows on the map (docs/adr/9102 §5, plan/veri.md VY-04): the rows pressed in Öznitelik
//! tablosu are read from the source with their geometry (`layers_rows::read`, their keys, off the interface's thread)
//! and their outlines drawn over the drawing in the selection's colour; Seçime yakınlaş frames them. The rows are not
//! the drawing's: the drawing's selection is left as it is.

use iced::widget::canvas::{self, Path, Stroke};
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{LayerNode, LayersRows};
use kentos_interaction::Vec2;
use kentos_render_wgpu::Camera;

use crate::app::{App, Message};
use crate::approved_layer::Approved;

/// The most rows outlined at once.
const MOST: u32 = 1000;

/// The selected rows' outlines and what they are of.
#[derive(Debug, Default)]
pub(crate) struct DataSelection {
    /// The layer and the keys the outlines are of.
    of: Option<(String, Vec<i64>)>,
    outlines: Option<Approved>,
    asked: u64,
}

/// What the read said.
#[derive(Debug, Clone)]
pub enum Event {
    Read(u64, Vec<kentos_commands::Entity>),
}

fn message(e: Event) -> Message {
    Message::Gis(crate::gis::Event::DataSelection(e))
}

impl App {
    /// After every message: the table's selection outlined when it changed (read off the thread).
    pub(crate) fn data_selection_tasks(&mut self) -> Task<Message> {
        let node: Option<LayerNode> = self.data_table_layer();
        let wanted = node.as_ref().and_then(|n| {
            let keys: Vec<i64> = self.gis.features.data.selected.iter().copied().collect();
            (!keys.is_empty()).then(|| (n.id.clone(), keys))
        });
        let sel = &mut self.gis.data_selection;
        if sel.of == wanted {
            return Task::none();
        }
        sel.of = wanted.clone();
        sel.outlines = None;
        sel.asked += 1;
        let (Some(node), Some((layer, keys)), Some(doc)) = (node, wanted, self.document.as_ref()) else {
            return Task::none();
        };
        let settings = doc.model.settings().clone();
        let asked = sel.asked;
        let input = LayersRows {
            layer,
            keys: Some(keys),
            fields: Some(Vec::new()),
            geometry: Some(true),
            limit: Some(MOST),
            ..LayersRows::default()
        };
        crate::db_connections::off_thread(
            move || {
                let rows = kentos_native_application::layers_rows::read(&node, &settings, &input, &|| false)
                    .map(|a| a.rows)
                    .unwrap_or_default();
                rows.iter()
                    .filter_map(|r| {
                        let g = kentos_data::wkt::read_wkt(r.wkt.as_deref()?).ok()?;
                        kentos_native_application::geopackage::object_of(&g, &node.id, 1, serde_json::Map::new())
                    })
                    .collect::<Vec<_>>()
            },
            move |objects| message(Event::Read(asked, objects)),
        )
    }

    pub(crate) fn data_selection_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Read(asked, objects) => {
                if asked == self.gis.data_selection.asked {
                    self.gis.data_selection.outlines = Some(Approved::of(&objects, asked));
                }
            }
        }
        Task::none()
    }

    /// Seçime yakınlaş on a data layer's table: the selected rows framed.
    pub(crate) fn zoom_to_data_selection(&mut self) {
        let Some(o) = self.gis.data_selection.outlines.as_ref() else {
            return;
        };
        let mut b = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
        for p in o.lines.iter().flatten().chain(&o.points) {
            b = [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])];
        }
        if !b.iter().all(|v| v.is_finite()) {
            return;
        }
        // A point alone is framed with room round it.
        let pad = ((b[2] - b[0]).max(b[3] - b[1]) * 0.1).max(10.0);
        self.navigating(|app| {
            app.zoom_to_bounds(&kentos_commands::Bounds {
                min_x: b[0] - pad,
                min_y: b[1] - pad,
                max_x: b[2] + pad,
                max_y: b[3] + pad,
            })
        });
    }

    /// The selected rows' outlines over the drawing, while the table shows their layer.
    pub(crate) fn data_selection_layer(&self, color: Color) -> Option<Element<'_, Message>> {
        let outlines = self.gis.data_selection.outlines.as_ref().filter(|o| o.objects > 0)?;
        self.gis.data_selection.of.as_ref()?;
        Some(
            canvas::Canvas::new(Layer {
                outlines,
                camera: self.viewport.camera,
                color,
            })
            .width(Fill)
            .height(Fill)
            .into(),
        )
    }
}

struct Layer<'a> {
    outlines: &'a Approved,
    camera: Camera,
    color: Color,
}

impl canvas::Program<Message> for Layer<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let screen = |p: [f64; 2]| {
            let s = self.camera.world_to_screen(Vec2::new(p[0], p[1]));
            Point::new(s[0] as f32, s[1] as f32)
        };
        let stroke = Stroke::default().with_width(2.5).with_color(self.color);
        for line in &self.outlines.lines {
            let path = Path::new(|b| {
                let mut pts = line.iter().map(|p| screen(*p));
                if let Some(first) = pts.next() {
                    b.move_to(first);
                    for p in pts {
                        b.line_to(p);
                    }
                }
            });
            frame.stroke(&path, stroke);
        }
        for p in &self.outlines.points {
            frame.stroke(&Path::circle(screen(*p), 6.0), stroke);
        }
        vec![frame.into_geometry()]
    }
}
