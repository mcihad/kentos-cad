//! İçine tıklayarak alan: the web's `BoundaryTool` (`apps/web/src/tools/areaTools.ts`),
//! step for step (docs/adr/0065). A click inside a region the visible line
//! work closes makes it an area on the active layer (AutoCAD's BOUNDARY,
//! Netcad's “alan oluştur”); closed groups inside it are holes unless Adalar
//! is off. The boundary set is every visible layer, or one picked by an
//! object of it (Sınır katmanı, K). One undo step, “Alan oluştur”, written
//! into the document as the web's is.

use std::collections::BTreeMap;

use kentos_contracts::EntityBase;
use kentos_domain::Slot;
use kentos_geometry_core::entity::polygon_ring;
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::region::net_area;
use kentos_geometry_core::ops::areas::polygon_of_area;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::{edit_geometry, entity_of};

use crate::Vec2;
use crate::faces;
use crate::format::Format;
use crate::log::Level;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{self, Context, Flow, Memory, Pointer, Preview, Tag, Tone, Tool};

/// The tool's id: its command is `tool.boundary`.
pub const ID: &str = "boundary";
pub const LABEL: &str = "İçine tıklayarak alan";

/// The boundary tool.
#[derive(Default)]
pub struct Boundary {
    picking_layer: bool,
    /// Only this layer's line work bounds the regions; none, every visible layer's.
    layer: Option<String>,
    layer_name: Option<String>,
    faces: Option<faces::Faces>,
    /// Where the cursor is and the region around it.
    hover: Option<(Vec2, Option<Area>)>,
    memory: Memory,
}

/// The layer new objects go to, said as the web's `writableLayer` says it:
/// none when it is locked; a hidden one with a warning.
fn writable_layer(cx: &mut Context<'_>) -> Option<String> {
    let layers = cx.doc.layers();
    let id = layers.active().to_owned();
    let name = layers.get(&id)?.name.clone();
    if layers.is_locked(&id) {
        cx.say(
            Level::Warn,
            format!(
                "“{name}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin."
            ),
        );
        return None;
    }
    if !layers.is_visible(&id) {
        cx.say(
            Level::Warn,
            format!("“{name}” katmanı gizli; çizilen nesne görünmeyecek."),
        );
    }
    Some(id)
}

impl Boundary {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.memory = *cx.memory;
        self.layer_name = self.layer.as_deref().map(|id| {
            cx.doc
                .layers()
                .get(id)
                .map_or_else(|| "—".to_owned(), |n| n.name.clone())
        });
    }

    fn face_at(&mut self, p: Vec2, cx: &Context<'_>) -> Option<Area> {
        let layer = self.layer.clone();
        faces::face_at(
            &mut self.faces,
            p,
            cx.memory.boundary_islands,
            layer.as_deref(),
            cx,
        )
    }

    /// The region around `p` becomes an area (the web's `pointerDown`).
    fn make(&mut self, p: Vec2, cx: &mut Context<'_>) {
        let Some(area) = self.face_at(p, cx) else {
            cx.say(
                Level::Warn,
                "Tıklanan yer kapalı bir bölgenin içinde değil. Bölgeyi saran çizgiler birleşmeli ya da kesişmeli; görünüm dışındaki çizgiler sayılmaz.",
            );
            return;
        };
        let Some(layer_id) = writable_layer(cx) else {
            return;
        };
        let Some(geometry) = edit_geometry(polygon_of_area(&area).shape) else {
            return;
        };
        let e = entity_of(
            &geometry,
            EntityBase {
                id: 0,
                layer_id,
                color: None,
                attrs: BTreeMap::new(),
                label: None,
                symbol: None,
            },
        );
        let written = cx
            .doc
            .transact("Alan oluştur", |doc| doc.add(e).map(|slot| vec![slot]));
        let slots: Vec<Slot> = match written {
            Ok(slots) => slots,
            Err(e) => {
                cx.say(Level::Error, e.to_string());
                return;
            }
        };
        cx.selection.set(slots);
        let holes = if area.holes.is_empty() {
            String::new()
        } else {
            format!(", {} ada (delik)", area.holes.len())
        };
        let size = cx.format().area(net_area(&area));
        cx.say(
            Level::Success,
            format!("Alan oluşturuldu: {size}{holes}."),
        );
    }
}

impl Tool for Boundary {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        if self.picking_layer {
            return Prompt::new(LABEL, "sınır olacak katmandan bir nesneye tıklayın")
                .option("Tüm katmanlar", "K");
        }
        Prompt::new(LABEL, "alanı oluşturulacak bölgenin içine tıklayın")
            .option_with(
                "Adalar",
                "A",
                if self.memory.boundary_islands {
                    "delik olur"
                } else {
                    "yok sayılır"
                },
            )
            .option_with(
                "Sınır katmanı",
                "K",
                self.layer_name.clone().unwrap_or_else(|| "tümü".to_owned()),
            )
    }

    fn point_count(&self) -> usize {
        0
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn snaps(&self) -> bool {
        false
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        if self.picking_layer {
            let hit = cx.spatial.pick(p.raw, cx.pick_tolerance());
            cx.selection.set_hover(hit);
            return;
        }
        let area = self.face_at(p.raw, cx);
        self.hover = Some((p.raw, area));
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        if self.picking_layer {
            let Some(layer) = cx
                .spatial
                .pick(p.raw, cx.pick_tolerance())
                .and_then(|s| cx.doc.get(s))
                .map(|e| e.base().layer_id.clone())
            else {
                cx.say(
                    Level::Warn,
                    "Sınır katmanını seçmek için bir nesneye tıklayın.",
                );
                return;
            };
            self.layer = Some(layer);
            self.picking_layer = false;
            cx.selection.set_hover(None);
            self.see(cx);
            return;
        }
        self.make(p.raw, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let done = match upper_tr(js_trim(text)).as_str() {
            "A" if !self.picking_layer => {
                cx.memory.boundary_islands = !cx.memory.boundary_islands;
                if let Some((at, _)) = self.hover {
                    let area = self.face_at(at, cx);
                    self.hover = Some((at, area));
                }
                true
            }
            "K" => {
                if self.picking_layer || self.layer.is_some() {
                    self.layer = None;
                    self.picking_layer = false;
                } else {
                    self.picking_layer = true;
                }
                cx.selection.set_hover(None);
                true
            }
            _ => false,
        };
        self.see(cx);
        done
    }

    /// Enter, Space or a quick right click leave the tool.
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    /// Esc leaves the layer picking first.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if !self.picking_layer {
            return false;
        }
        self.picking_layer = false;
        cx.selection.set_hover(None);
        true
    }

    /// The tool takes nothing back itself: Ctrl+Z is the drawing's.
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    /// The region under the cursor filled lightly, its area and holes by it.
    fn preview(&self, format: &Format) -> Preview {
        let Some((at, Some(area))) = self.hover.as_ref().filter(|_| !self.picking_layer) else {
            return Preview::default();
        };
        let mut lines = vec![format.area(net_area(area))];
        if !area.holes.is_empty() {
            lines.push(format!("{} ada", area.holes.len()));
        }
        Preview {
            areas: vec![tool::Area {
                rings: std::iter::once(&area.outer)
                    .chain(&area.holes)
                    .map(|r| polygon_ring(&r.pts, r.bulges.as_deref()))
                    .collect(),
                fill: 0.16,
                width: 2.0,
                dash: None,
                fill_tone: Tone::Accent,
            }],
            tag: Some(Tag { at: *at, lines }),
            ..Preview::default()
        }
    }
}
