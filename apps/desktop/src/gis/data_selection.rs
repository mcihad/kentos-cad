//! A data layer's selected rows on the map (docs/adr/9102 §5, plan/veri.md VY-04): the rows pressed in Öznitelik
//! tablosu are read from the source with their geometry and values (`layers_rows::read`, their keys, off the
//! interface's thread) and their outlines drawn over the drawing in the selection's colour; Seçime yakınlaş frames
//! them. The rows are not the drawing's: the drawing's selection is left as it is. Öznitelikler and Koordinat listesi
//! show the same rows, by their layer and key, read without taking them (docs/adr/9326).
//!
//! The rows are read again when the layer's changes are written to the source or let go (docs/adr/9346): until the
//! answer comes Öznitelikler says it reads them again, not the old values as the source's. A row taken into the edit
//! session shows its value waiting to be written, said so, as the table shows it.

use iced::widget::canvas::{self, Path, Stroke};
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{LayerNode, LayersRows};
use kentos_interaction::Vec2;
use kentos_render_wgpu::Camera;

use crate::app::{App, Message};
use crate::approved_layer::Approved;

/// The most rows outlined at once.
const MOST: u32 = 1000;

/// A selected row as its source holds it: its key, its values and its geometry as an object of the layer (none for a
/// table without one).
#[derive(Debug, Clone)]
pub struct SourceRow {
    pub key: i64,
    pub attrs: std::collections::BTreeMap<String, String>,
    pub object: Option<kentos_commands::Entity>,
}

/// The selected rows' outlines and what they are of.
#[derive(Debug, Default)]
pub(crate) struct DataSelection {
    /// The layer and the keys the outlines are of.
    of: Option<(String, Vec<i64>)>,
    outlines: Option<Approved>,
    /// The rows read, in the keys' order the source answered (docs/adr/9326).
    rows: Vec<SourceRow>,
    asked: u64,
    /// The sources' writes the rows were read after ([`kentos_domain::Document::data_written`]).
    written: u64,
    /// The rows are read again after a write: shown, said as being read.
    stale: bool,
}

/// What the read said.
#[derive(Debug, Clone)]
pub enum Event {
    Read(u64, Vec<SourceRow>),
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
        let written = self.document.as_ref().map_or(0, |d| d.model.data_written());
        let sel = &mut self.gis.data_selection;
        let same = sel.of == wanted;
        if same && (wanted.is_none() || sel.written == written) {
            return Task::none();
        }
        sel.of = wanted.clone();
        sel.written = written;
        if same {
            // Written or let go: the same rows read again, the old values said as such until they come.
            sel.stale = true;
        } else {
            sel.outlines = None;
            sel.rows.clear();
            sel.stale = false;
        }
        sel.asked += 1;
        let (Some(node), Some((layer, keys)), Some(doc)) = (node, wanted, self.document.as_ref())
        else {
            return Task::none();
        };
        let settings = doc.model.settings().clone();
        let asked = sel.asked;
        let input = LayersRows {
            layer,
            keys: Some(keys),
            geometry: Some(true),
            limit: Some(MOST),
            ..LayersRows::default()
        };
        crate::db_connections::off_thread(
            move || {
                let rows =
                    kentos_native_application::layers_rows::read(&node, &settings, &input, &|| {
                        false
                    })
                    .map(|a| a.rows)
                    .unwrap_or_default();
                rows.into_iter()
                    .map(|r| {
                        let attrs: serde_json::Map<String, serde_json::Value> = r
                            .attrs
                            .iter()
                            .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                            .collect();
                        let object = r
                            .wkt
                            .as_deref()
                            .and_then(|w| kentos_data::wkt::read_wkt(w).ok())
                            .and_then(|g| {
                                let id = u32::try_from(r.key).unwrap_or(1);
                                kentos_native_application::geopackage::object_of(
                                    &g, &node.id, id, attrs,
                                )
                            });
                        SourceRow {
                            key: r.key,
                            attrs: r.attrs,
                            object,
                        }
                    })
                    .collect::<Vec<_>>()
            },
            move |objects| message(Event::Read(asked, objects)),
        )
    }

    pub(crate) fn data_selection_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Read(asked, rows) => {
                if asked == self.gis.data_selection.asked {
                    let objects: Vec<kentos_commands::Entity> =
                        rows.iter().filter_map(|r| r.object.clone()).collect();
                    self.gis.data_selection.outlines = Some(Approved::of(&objects, asked));
                    self.gis.data_selection.rows = rows;
                    self.gis.data_selection.stale = false;
                    // An export's Seçili takes them (shell/exchange/source_rows.rs).
                    self.export_sources_read();
                }
            }
        }
        Task::none()
    }

    /// The rows chosen in a data layer's table for an export's Seçili (docs/adr/9338): the layer and its rows' objects
    /// as read, none while they are read; none at all without a chosen row.
    pub(crate) fn chosen_source_rows(
        &self,
    ) -> Option<(String, Option<Vec<kentos_commands::Entity>>)> {
        let sel = &self.gis.data_selection;
        let (layer, _) = sel.of.as_ref()?;
        let read = sel
            .outlines
            .is_some()
            .then(|| sel.rows.iter().filter_map(|r| r.object.clone()).collect());
        Some((layer.clone(), read))
    }

    /// The data layer's rows selected in its table and read from its source, with the layer's id (docs/adr/9326):
    /// what Öznitelikler and Koordinat listesi show while the drawing has nothing selected.
    pub(crate) fn selected_source_rows(&self) -> Option<(&str, &[SourceRow])> {
        let sel = &self.gis.data_selection;
        let (layer, _) = sel.of.as_ref()?;
        (!sel.rows.is_empty()).then_some((layer.as_str(), sel.rows.as_slice()))
    }

    /// The first selected source row in words for Öznitelikler (docs/adr/9326): the heading with how many rows, the
    /// layer and the key, the values by the layer's fields (a code by its label), and what is said under them. A row
    /// taken into the edit session shows its values waiting to be written (docs/adr/9346), as the table does; rows read
    /// again after a write are said so.
    pub(crate) fn source_row_lines(&self) -> Option<(String, Vec<(String, String)>, String)> {
        let (layer, rows) = self.selected_source_rows()?;
        let doc = &self.document.as_ref()?.model;
        let node = doc.layers().get(layer)?;
        let first = rows.first()?;
        let waiting = doc
            .taken_objects(layer)
            .into_iter()
            .find(|(k, _)| *k == first.key)
            .map(|(_, e)| e.base().attrs.clone())
            .filter(|now| *now != first.attrs);
        let deleted = doc.deleted_keys(layer).contains(&first.key)
            || doc
                .taken_uid(layer, first.key)
                .is_some_and(|u| doc.slot_of(u).is_none());
        let attrs = waiting.as_ref().unwrap_or(&first.attrs);
        let key = node.data.as_ref().map_or("anahtar", |d| d.key.as_str());
        let mut lines = vec![
            ("Katman".to_owned(), node.name.clone()),
            (key.to_owned(), first.key.to_string()),
        ];
        for f in &node.fields {
            let raw = attrs.get(&f.name).cloned().unwrap_or_default();
            let shown = if f.values.as_ref().is_some_and(|v| !v.is_empty()) && !raw.is_empty() {
                kentos_commands::display_value(f, &raw)
            } else {
                raw
            };
            lines.push((f.label().to_owned(), shown));
        }
        let many = if rows.len() > 1 {
            "İlki gösteriliyor. "
        } else {
            ""
        };
        let state = if self.gis.data_selection.stale {
            "Kaynaktan yeniden okunuyor…"
        } else if deleted {
            "Silinecek: kaynağa yazılmadı."
        } else if waiting.is_some() {
            "Bekleyen değişiklik: kaynağa yazılmadı."
        } else {
            "Kaynaktan okundu; çizime alınmadı."
        };
        Some((
            format!("Veri satırı ({})", rows.len()),
            lines,
            format!("{many}{state}"),
        ))
    }

    /// Whether a data layer's selected rows are outlined on the map (Seçime yakınlaştır frames them).
    pub(crate) fn data_rows_outlined(&self) -> bool {
        self.gis
            .data_selection
            .outlines
            .as_ref()
            .is_some_and(|o| o.objects > 0)
    }

    /// Seçime yakınlaş on a data layer's table: the selected rows framed.
    pub(crate) fn zoom_to_data_selection(&mut self) {
        let Some(o) = self.gis.data_selection.outlines.as_ref() else {
            return;
        };
        let mut b = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for p in o.lines.iter().flatten().chain(&o.points) {
            b = [
                b[0].min(p[0]),
                b[1].min(p[1]),
                b[2].max(p[0]),
                b[3].max(p[1]),
            ];
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
        let slot = &self.gpu_layers.selection;
        let Some(outlines) = self
            .gis
            .data_selection
            .outlines
            .as_ref()
            .filter(|o| o.objects > 0)
        else {
            slot.clear();
            return None;
        };
        if self.gis.data_selection.of.is_none() {
            slot.clear();
            return None;
        }
        Some(
            canvas::Canvas::new(Layer {
                outlines,
                camera: self.viewport.camera,
                color,
                gpu: crate::glyph_layer::on().then(|| slot.clone()),
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
    /// Where it goes for the area's renderer (plan/render.md RM-14); none: drawn on the canvas.
    gpu: Option<crate::glyph_layer::GlyphSlot>,
}

impl Layer<'_> {
    /// The selected rows' outlines, and a ring a point.
    fn paint(&self, frame: &mut impl crate::gpu_frame::Paint) {
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
    }

    /// What its picture depends on: the outlines and the view.
    fn key(&self, bounds: Rectangle) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (
            self.outlines.version,
            self.outlines.objects,
            self.outlines.lines.len(),
        )
            .hash(&mut h);
        for p in [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
        ] {
            for c in self.camera.world_to_screen(p) {
                c.to_bits().hash(&mut h);
            }
        }
        for c in [
            bounds.width,
            bounds.height,
            self.color.r,
            self.color.g,
            self.color.b,
            self.color.a,
        ] {
            c.to_bits().hash(&mut h);
        }
        h.finish()
    }
}

impl canvas::Program<Message> for Layer<'_> {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        if let Some(slot) = &self.gpu {
            crate::gpu_frame::publish(slot, self.key(bounds), self.camera, bounds.size(), |f| {
                self.paint(f)
            });
            return Vec::new();
        }
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        self.paint(&mut frame);
        vec![frame.into_geometry()]
    }
}
