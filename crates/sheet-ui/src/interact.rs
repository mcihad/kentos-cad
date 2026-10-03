//! The pointer on the paper (design §5): a press chooses (an item, a handle
//! of the chosen one, or nothing: a box), a drag moves, resizes, turns or
//! draws a new item with the core's snapping, a release writes one
//! operation. While a drag is under way the book is shown as it would be
//! after it ([`Preview`]): the core lays it out again, so constraints, text
//! and grids follow live.

use kentos_sheet::display::{self, RenderMode};
use kentos_sheet::hit::{
    HandleHit, HandleQuery, HitQuery, PointQuery, RectMode, RectQuery, hit_test,
};
use kentos_sheet::model::ItemId;
use kentos_sheet::ops::{self, AddItems, MoveItems, Op, ResizeItem, RotateItems};
use kentos_sheet::profile::{NewItem, new_item};
use kentos_sheet::snap::{SnapOptions, SnapSession, TOLERANCE_PX, snap_rotation};
use kentos_sheet::units::{RectUm, round_um};

use crate::designer::{Designer, Drag, Effect, Preview};
use crate::message::{StageEvent, Tool};
use crate::paint;

/// How far above the top edge the turning handle stands, pixels.
pub(crate) const ROTATE_OFFSET_PX: f64 = 20.0;
/// How near a handle a press takes it, pixels.
const HANDLE_PX: f64 = 6.0;
/// A new item drawn smaller than this is given its kind's size instead, millimetres.
const CLICK_MM: f64 = 2.0;

/// The keys held during a drag.
#[derive(Clone, Copy, Debug, Default)]
struct Keys {
    shift: bool,
    ctrl: bool,
    alt: bool,
}

fn um(p: [f64; 2]) -> [i32; 2] {
    [round_um(p[0]), round_um(p[1])]
}

impl Designer {
    /// Pixels to micrometres at the current zoom.
    pub(crate) fn px_um(&self, px: f64) -> f64 {
        let paper = self.open_sheet().map(|s| s.page.size);
        paper.map_or(px * 1000.0, |p| px / self.view.xf(self.stage, p).k)
    }

    pub(crate) fn stage_event(&mut self, e: StageEvent) -> Vec<Effect> {
        let Some(paper) = self.open_sheet().map(|s| s.page.size) else {
            return Vec::new();
        };
        match e {
            StageEvent::Move {
                at,
                size,
                shift,
                ctrl,
                alt,
            } => {
                self.stage = size;
                self.hover = Some(at);
                if self.drag.is_some() {
                    self.drag_to(at, Keys { shift, ctrl, alt });
                } else {
                    self.hovered_item = self.hit(at).map(|(_, top)| top);
                }
            }
            StageEvent::Leave => {
                self.hover = None;
                self.hovered_item = None;
            }
            StageEvent::Resize { size } => self.stage = size,
            StageEvent::Press {
                at,
                size,
                shift,
                ctrl,
                alt,
                ..
            } => {
                self.stage = size;
                self.press(at, shift, ctrl, alt);
            }
            StageEvent::Release {
                at,
                shift,
                ctrl,
                alt,
            } => return self.release(at, Keys { shift, ctrl, alt }),
            StageEvent::DoubleClick { at } => {
                // Into a group: the item under the pointer itself.
                if let Some((item, _)) = self.hit(at) {
                    self.select(vec![item]);
                }
            }
            StageEvent::Zoom { factor, px, size } => {
                self.stage = size;
                self.view = self.view.zoomed(factor, px, size, paper);
                self.reset_caches();
            }
            StageEvent::Pan { dx, dy, size } => {
                self.stage = size;
                self.view = self.view.panned(dx, dy, size, paper);
                self.reset_caches();
            }
        }
        Vec::new()
    }

    /// The item under a paper point and the outermost group it is in.
    fn hit(&self, at: [f64; 2]) -> Option<(ItemId, ItemId)> {
        let sheet = self.open.as_deref()?;
        let q = HitQuery::Point(PointQuery {
            at: um(at),
            tolerance: round_um(self.px_um(3.0)),
        });
        hit_test(&self.book, sheet, &q)
            .ok()?
            .hits
            .into_iter()
            .next()
            .map(|h| (h.item, h.top))
    }

    fn press(&mut self, at: [f64; 2], shift: bool, ctrl: bool, alt: bool) {
        let Some(sheet) = self.open.clone() else {
            return;
        };
        self.preview = None;
        let tool = if self.space {
            Tool::Hand
        } else {
            self.tool.clone()
        };
        match tool {
            Tool::Hand => {
                self.drag = Some(Drag::Pan);
                return;
            }
            Tool::Add { tool, preset } => {
                self.drag = Some(Drag::Create {
                    tool,
                    preset,
                    start: at,
                    end: at,
                });
                return;
            }
            Tool::Select => {}
        }
        // A handle of the one chosen item.
        if let [only] = self.selection.as_slice()
            && let Some(item) = self.book.item(only).filter(|i| !i.locked)
        {
            let q = HitQuery::Handle(HandleQuery {
                item: only.clone(),
                at: um(at),
                tolerance: round_um(self.px_um(HANDLE_PX)),
                rotate_offset: round_um(self.px_um(ROTATE_OFFSET_PX)),
            });
            if let Some(h) = hit_test(&self.book, &sheet, &q).ok().and_then(|h| h.handle) {
                let id = item.id.clone();
                match h.resize() {
                    Some(handle) => {
                        if let Ok(session) = SnapSession::new(
                            &self.book,
                            &sheet,
                            std::slice::from_ref(&id),
                            SnapOptions::default(),
                        ) {
                            self.drag = Some(Drag::Resize {
                                id,
                                handle,
                                session,
                            });
                        }
                    }
                    None => {
                        debug_assert_eq!(h, HandleHit::Rotate);
                        let c = item.frame.center();
                        self.drag = Some(Drag::Rotate {
                            ids: vec![id],
                            center: c,
                            start_angle: (at[1] - c[1]).atan2(at[0] - c[0]),
                            base: item.rotation,
                        });
                    }
                }
                return;
            }
        }
        match self.hit(at) {
            Some((item, top)) => {
                // Ctrl reaches into a group; Shift adds or takes away.
                let id = if ctrl { item } else { top };
                if shift {
                    if let Some(i) = self.selection.iter().position(|x| *x == id) {
                        self.selection.remove(i);
                        self.after_select();
                        return;
                    }
                    self.selection.push(id);
                    self.after_select();
                } else if !self.selection.contains(&id) {
                    self.select(vec![id]);
                }
                let ids: Vec<ItemId> = self
                    .chosen()
                    .iter()
                    .filter(|i| !i.locked)
                    .map(|i| i.id.clone())
                    .collect();
                if ids.is_empty() {
                    return;
                }
                let options = SnapOptions {
                    grid: None,
                    ..SnapOptions::default()
                };
                if let Ok(session) = SnapSession::new(&self.book, &sheet, &ids, options) {
                    self.drag = Some(Drag::Move {
                        ids,
                        start: at,
                        session,
                        moved: false,
                    });
                }
                let _ = alt;
            }
            None => {
                if !shift {
                    self.select(Vec::new());
                }
                self.drag = Some(Drag::Band {
                    start: at,
                    end: at,
                    add: shift,
                });
            }
        }
    }

    /// The operation a drag would write now, and what it snapped to.
    fn drag_ops(&mut self, at: [f64; 2], keys: Keys) -> Option<Preview> {
        let Keys { shift, ctrl, alt } = keys;
        // Ctrl held: no snapping.
        let tol = if ctrl {
            0
        } else {
            round_um(self.px_um(f64::from(TOLERANCE_PX)))
        };
        let mut preview = Preview::default();
        match self.drag.as_mut()? {
            Drag::Move {
                ids,
                start,
                session,
                moved,
            } => {
                let asked = [round_um(at[0] - start[0]), round_um(at[1] - start[1])];
                if asked == [0, 0] && !*moved {
                    return None;
                }
                *moved = true;
                let delta = if ctrl {
                    asked
                } else {
                    let r = session.query(asked, tol);
                    preview.lines = r.lines;
                    preview.gaps = r.gaps;
                    preview.spacing = r.spacing;
                    r.delta
                };
                preview.ops = vec![Op::MoveItems(MoveItems {
                    ids: ids.clone(),
                    delta,
                })];
            }
            Drag::Resize {
                id,
                handle,
                session,
            } => {
                let r = session.query_resize(*handle, um(at), tol, shift);
                preview.lines = r.lines;
                preview.ops = vec![Op::ResizeItem(ResizeItem {
                    id: id.clone(),
                    handle: *handle,
                    to: r.to,
                    keep_aspect: shift,
                    from_center: alt,
                })];
            }
            Drag::Rotate {
                ids,
                center,
                start_angle,
                base,
            } => {
                let now = (at[1] - center[1]).atan2(at[0] - center[0]);
                let turned = ((now - *start_angle).to_degrees() * 1000.0).round() as i64;
                let target = snap_rotation(
                    kentos_sheet::units::norm_mdeg(i64::from(*base) + turned),
                    shift,
                );
                let angle = kentos_sheet::units::norm_mdeg(i64::from(target) - i64::from(*base));
                if angle == 0 {
                    return None;
                }
                preview.ops = vec![Op::RotateItems(RotateItems {
                    ids: ids.clone(),
                    angle,
                    around: None,
                })];
            }
            Drag::Band { end, .. } | Drag::Create { end, .. } => {
                *end = at;
                return None;
            }
            Drag::Pan => return None,
        }
        Some(preview)
    }

    fn drag_to(&mut self, at: [f64; 2], keys: Keys) {
        let Some(mut preview) = self.drag_ops(at, keys) else {
            return;
        };
        if let (Some(sheet), Ok(applied)) =
            (self.open.clone(), ops::apply_all(&self.book, &preview.ops))
        {
            let inputs = self.inputs(RenderMode::Design);
            if let Ok((list, _)) = display::build(&applied.book, &sheet, &inputs) {
                preview.plan = paint::plan(&list);
                preview.list = Some(list);
            }
            preview.book = Some(applied.book);
        }
        self.preview = Some(preview);
        self.reset_caches();
    }

    fn release(&mut self, at: [f64; 2], keys: Keys) -> Vec<Effect> {
        let Some(drag) = self.drag.take() else {
            return Vec::new();
        };
        let preview = self.preview.take();
        match drag {
            Drag::Move { .. } | Drag::Resize { .. } | Drag::Rotate { .. } => {
                // The last pointer place counts, with the keys held at release.
                self.drag = Some(drag);
                let ops = self
                    .drag_ops(at, keys)
                    .map(|p| p.ops)
                    .or(preview.map(|p| p.ops));
                self.drag = None;
                match ops {
                    Some(ops) if !ops.is_empty() => {
                        self.commit(ops);
                    }
                    _ => self.reset_caches(),
                }
            }
            Drag::Band { start, end: _, add } => {
                let rect = RectUm::from_edges(
                    round_um(start[0].min(at[0])) as i64,
                    round_um(start[1].min(at[1])) as i64,
                    round_um(start[0].max(at[0])) as i64,
                    round_um(start[1].max(at[1])) as i64,
                );
                if rect.width > 0 || rect.height > 0 {
                    let mode = if at[0] >= start[0] {
                        RectMode::Contain
                    } else {
                        RectMode::Intersect
                    };
                    if let Some(sheet) = self.open.as_deref()
                        && let Ok(h) =
                            hit_test(&self.book, sheet, &HitQuery::Rect(RectQuery { rect, mode }))
                    {
                        let mut ids: Vec<ItemId> = if add {
                            self.selection.clone()
                        } else {
                            Vec::new()
                        };
                        for hit in h.hits {
                            if !ids.contains(&hit.top) {
                                ids.push(hit.top);
                            }
                        }
                        self.select(ids);
                    }
                }
                self.reset_caches();
            }
            Drag::Create {
                tool,
                preset,
                start,
                end: _,
            } => {
                self.create(&tool, preset, start, at);
            }
            Drag::Pan => {}
        }
        Vec::new()
    }

    /// A new item of a tool in the box drawn (or its kind's size where it was clicked), chosen; the tool goes back to choosing.
    fn create(&mut self, tool: &str, preset: Option<String>, a: [f64; 2], b: [f64; 2]) {
        let Some(owner) = self.owner() else {
            return;
        };
        let (w, h) = ((b[0] - a[0]).abs(), (b[1] - a[1]).abs());
        let frame = if w < CLICK_MM * 1000.0 && h < CLICK_MM * 1000.0 {
            let (dw, dh) = default_size(tool);
            RectUm::new(
                round_um(a[0]),
                round_um(a[1]),
                round_um(dw * 1000.0),
                round_um(dh * 1000.0),
            )
        } else {
            RectUm::new(
                round_um(a[0].min(b[0])),
                round_um(a[1].min(b[1])),
                round_um(w).max(1000),
                round_um(h).max(1000),
            )
        };
        // The new item reads the first map (a scale bar's, a legend's), or the map under it.
        let link = self
            .items()
            .iter()
            .rev()
            .find(|i| matches!(i.kind, kentos_sheet::kinds::ItemKind::Map(_)))
            .map(|i| i.id.clone());
        let label = self
            .profile
            .groups
            .iter()
            .flat_map(|g| g.tools.iter())
            .find(|t| t.id == tool)
            .map_or_else(|| tool.to_owned(), |t| t.label.clone());
        let same = self
            .items()
            .iter()
            .filter(|i| kentos_sheet::profile::tool_of(i) == tool)
            .count();
        let name = if same == 0 {
            label
        } else {
            format!("{label} {}", same + 1)
        };
        let id = self.new_id("o");
        let req = NewItem {
            tool: tool.to_owned(),
            preset,
            id: id.clone(),
            name,
            frame,
            link,
        };
        match new_item(self.ctx.workspace, &self.ctx.capabilities, &req) {
            Ok(mut item) => {
                // A new map looks where the drawing area does.
                if let (kentos_sheet::kinds::ItemKind::Map(m), Some(c)) =
                    (&mut item.kind, self.ctx.center)
                    && let kentos_sheet::kinds::MapView::Fixed(f) = &mut m.view
                    && f.center.is_none()
                {
                    f.center = Some(c);
                }
                if self.commit(vec![Op::AddItems(AddItems {
                    to: owner,
                    items: vec![item],
                    index: None,
                })]) {
                    self.select(vec![id]);
                }
            }
            Err(e) => self.error = Some(e.message),
        }
        self.tool = Tool::Select;
    }
}

/// A clicked new item's size by its tool, millimetres.
fn default_size(tool: &str) -> (f64, f64) {
    match tool {
        "map" | "overviewMap" => (120.0, 90.0),
        "text" => (60.0, 12.0),
        "legend" => (50.0, 60.0),
        "scaleBar" => (60.0, 12.0),
        "northArrow" => (14.0, 20.0),
        "table" | "attributeTable" | "coordinateList" => (90.0, 40.0),
        "titleBlock" => (120.0, 40.0),
        "picture" => (40.0, 30.0),
        "line" => (60.0, 1.0),
        _ => (40.0, 30.0),
    }
}
