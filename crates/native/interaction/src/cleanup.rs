//! Çizimi temizle (docs/adr/0140): finds what the drawing carries twice or
//! empty, shows it, and on a confirm removes it in one undo step.
//!
//! - **Scope:** the selection; with nothing selected, the whole drawing. Objects
//!   on a locked layer are left out, and so, in the whole drawing, are those
//!   on a hidden one (what is not seen is not cleaned).
//! - **Found:** an object repeated exactly (the same kind and geometry to
//!   1e-9 m on the same layer; the first stays), an object with no length or
//!   no radius, and a vertex repeated in a row. Nothing is snapped and no
//!   tolerance is widened (CLAUDE.md §23.3).
//! - **Shown first:** the repeats dashed in the danger colour, a cross where an
//!   empty object or a repeated vertex is, and the count on the command line.
//!   Nothing to clean is said and the tool leaves. Enter, the Uygula button
//!   or a quick right click cleans; Esc leaves.
//!
//! Written through `cad.entities.edit` (`cleanup`): repeats and empty objects
//! removed, cleaned paths updated. The findings are the shared core's
//! (`cleanup_findings`).

use kentos_contracts::{EditOperation, Entity, EntityEdit};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::api::json::Json;
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape};
use kentos_geometry_core::ops::split::{Findings, cleanup_findings};
use kentos_native_application::geometry::shape;

use crate::Vec2;
use crate::edge::{self, Outline};
use crate::format::Format;
use crate::log::Level;
use crate::modify::MAX_GHOSTS;
use crate::prompt::Prompt;
use crate::tool::{Context, Cursor, Flow, Marker, MarkerShape, Preview, Tone, Tool};

/// Çizimi temizle's id: its command is `tool.cleanup`.
pub const ID: &str = "cleanup";
pub const LABEL: &str = "Çizimi temizle";

/// Two points closer than this are the same point (the core's rule).
const SAME: f64 = 1e-9;

/// The tool.
#[derive(Clone, Debug, Default)]
pub struct Cleanup {
    /// The objects it looks at, in the order it looked.
    scope: Vec<Slot>,
    /// Whether that is the whole drawing (else the selection).
    whole: bool,
    /// What it found, and how to draw it.
    found: Option<Found>,
    done: bool,
}

/// What Çizimi temizle found, as slots.
#[derive(Clone, Debug, Default)]
struct Found {
    repeats: Vec<Slot>,
    empty: Vec<Slot>,
    cleaned: Vec<(Slot, Shape)>,
    vertices: usize,
    drawn: Preview,
}

impl Found {
    fn is_empty(&self) -> bool {
        self.repeats.is_empty() && self.empty.is_empty() && self.cleaned.is_empty()
    }

    /// `3 yinelenen, 1 boş nesne, 5 tekrarlanan köşe`.
    fn text(&self) -> String {
        let mut parts = Vec::new();
        if !self.repeats.is_empty() {
            parts.push(format!("{} yinelenen", self.repeats.len()));
        }
        if !self.empty.is_empty() {
            parts.push(format!("{} boş nesne", self.empty.len()));
        }
        if self.vertices > 0 {
            parts.push(format!("{} tekrarlanan köşe", self.vertices));
        }
        parts.join(", ")
    }
}

impl Cleanup {
    pub fn new() -> Self {
        Self::default()
    }

    /// The objects to look at: the selection, or the whole drawing when none
    /// is selected. Locked layers are left out (said), and in the whole
    /// drawing hidden ones too.
    fn scope(&mut self, cx: &mut Context<'_>) {
        let doc = &*cx.doc;
        self.whole = cx.selection.is_empty();
        let all: Vec<Slot> = if self.whole {
            doc.entities().map(|e| Slot(e.base().id)).collect()
        } else {
            cx.selection
                .ids()
                .iter()
                .copied()
                .filter(|slot| doc.get(*slot).is_some())
                .collect()
        };
        let layers = doc.layers();
        let takes = |e: &Entity| {
            let layer = &e.base().layer_id;
            !layers.is_locked(layer) && (!self.whole || layers.is_visible(layer))
        };
        self.scope = all
            .iter()
            .copied()
            .filter(|slot| doc.get(*slot).is_some_and(takes))
            .collect();
        let left = all.len() - self.scope.len();
        if left > 0 {
            let why = if self.whole {
                "kilitli ya da gizli katmanda olduğu için dışarıda bırakıldı"
            } else {
                "kilitli katmanda olduğu için atlandı"
            };
            cx.say(Level::Warn, format!("{left} nesne {why}."));
        }
    }

    /// The findings for the scope as the drawing is now.
    fn find(&self, doc: &Document, draw: bool) -> Found {
        // The core names each object by its `id` (the slot) and its layer.
        let list: Vec<(Slot, CoreEntity)> = self
            .scope
            .iter()
            .filter_map(|slot| {
                let e = doc.get(*slot)?;
                Some((
                    *slot,
                    CoreEntity {
                        shape: shape(e),
                        rest: vec![
                            ("id".to_owned(), Json::Num(f64::from(slot.0))),
                            ("layerId".to_owned(), Json::Str(e.base().layer_id.clone())),
                        ],
                    },
                ))
            })
            .collect();
        let entities: Vec<CoreEntity> = list.iter().map(|(_, e)| e.clone()).collect();
        let Findings {
            repeats,
            empty,
            cleaned,
            vertices,
        } = cleanup_findings(&entities);
        let slot_of = |id: &Json| match id {
            Json::Num(n) if *n >= 0.0 && n.fract() == 0.0 => Some(Slot(*n as u32)),
            _ => None,
        };
        let shape_of = |slot: Slot| list.iter().find(|(s, _)| *s == slot).map(|(_, e)| &e.shape);
        let repeats: Vec<Slot> = repeats.iter().filter_map(slot_of).collect();
        let empty: Vec<Slot> = empty.iter().filter_map(slot_of).collect();
        let cleaned: Vec<(Slot, Shape)> = cleaned
            .into_iter()
            .filter_map(|e| {
                let slot = e
                    .rest
                    .iter()
                    .find(|(k, _)| k == "id")
                    .and_then(|(_, v)| slot_of(v))?;
                Some((slot, e.shape))
            })
            .collect();
        let mut found = Found {
            repeats,
            empty,
            cleaned,
            vertices,
            drawn: Preview::default(),
        };
        if draw {
            found.drawn = Self::drawn(&found, shape_of);
        }
        found
    }

    /// The repeats dashed in danger, a cross at each empty object and at each
    /// vertex that repeats.
    fn drawn<'a>(found: &Found, shape_of: impl Fn(Slot) -> Option<&'a Shape>) -> Preview {
        let mut out = Outline::default();
        let mut markers = Vec::new();
        let cross = |at: Vec2| Marker {
            at,
            shape: MarkerShape::Cross(5.0),
            tone: Tone::Danger,
        };
        for slot in found.repeats.iter().take(MAX_GHOSTS) {
            if let Some(s) = shape_of(*slot) {
                out = out.and(Outline::of(s, Some([5.0, 3.0]), 2.0, Tone::Danger));
            }
        }
        for slot in found.empty.iter().take(MAX_GHOSTS) {
            if let Some(at) = shape_of(*slot).and_then(anchor) {
                markers.push(cross(at));
            }
        }
        for (slot, _) in found.cleaned.iter().take(MAX_GHOSTS) {
            if let Some(s) = shape_of(*slot) {
                markers.extend(repeated_vertices(s).into_iter().map(cross));
            }
        }
        Preview {
            strokes: out.strokes,
            marks: out.marks,
            markers,
            ..Preview::default()
        }
    }

    /// Writes the cleaning as one step: repeats and empty objects removed,
    /// cleaned paths updated.
    fn clean(&mut self, cx: &mut Context<'_>) -> Flow {
        let found = self.find(cx.doc, false);
        if found.is_empty() {
            cx.say(Level::Info, "Çizimi temizle: temizlenecek bir şey kalmadı.");
            return Flow::Exit;
        }
        let mut changes: Vec<EntityEdit> = found
            .repeats
            .iter()
            .chain(&found.empty)
            .map(|slot| EntityEdit::Remove {
                uid: edge::uid(cx.doc, *slot),
            })
            .collect();
        for (slot, shape) in &found.cleaned {
            if let Some(geometry) = edge::geometry(shape) {
                changes.push(EntityEdit::Update {
                    uid: edge::uid(cx.doc, *slot),
                    geometry,
                });
            }
        }
        if edge::write(EditOperation::Cleanup, changes, cx).is_none() {
            return Flow::Stay;
        }
        let doc = &*cx.doc;
        cx.selection.retain(|slot| doc.get(slot).is_some());
        let mut said = Vec::new();
        if !found.repeats.is_empty() {
            said.push(format!("{} yinelenen", found.repeats.len()));
        }
        if !found.empty.is_empty() {
            said.push(format!("{} boş nesne", found.empty.len()));
        }
        let mut text = if said.is_empty() {
            String::new()
        } else {
            format!("{} silindi", said.join(", "))
        };
        if found.vertices > 0 {
            if !text.is_empty() {
                text.push_str("; ");
            }
            text.push_str(&format!("{} tekrarlanan köşe atıldı", found.vertices));
        }
        cx.say(Level::Success, format!("Çizimi temizle: {text}."));
        self.done = true;
        Flow::Exit
    }
}

/// A place to mark an empty object: where it is.
fn anchor(s: &Shape) -> Option<Vec2> {
    match s {
        Shape::Line { a, .. } => Some(*a),
        Shape::Circle { c, .. } | Shape::Arc { c, .. } => Some(*c),
        Shape::Text { p, .. } => Some(*p),
        Shape::Polyline { pts, .. } | Shape::Polygon { pts, .. } | Shape::Spline { pts, .. } => {
            pts.first().copied()
        }
        _ => None,
    }
}

/// The vertices of a path or an area that repeat the one before (a closed
/// ring's last that repeats its first).
fn repeated_vertices(s: &Shape) -> Vec<Vec2> {
    let ring = |pts: &[Vec2], closed: bool, out: &mut Vec<Vec2>| {
        let same = |a: Vec2, b: Vec2| (a.x - b.x).abs() <= SAME && (a.y - b.y).abs() <= SAME;
        for w in pts.windows(2) {
            if same(w[0], w[1]) {
                out.push(w[1]);
            }
        }
        if let (true, Some(first), Some(last)) = (closed, pts.first(), pts.last())
            && pts.len() > 1
            && same(*first, *last)
        {
            out.push(*last);
        }
    };
    let mut out = Vec::new();
    match s {
        Shape::Polyline { pts, .. } => ring(pts, false, &mut out),
        Shape::Polygon { pts, holes, .. } => {
            ring(pts, true, &mut out);
            for h in holes.iter().flatten() {
                ring(&h.pts, true, &mut out);
            }
        }
        _ => {}
    }
    out
}

impl Tool for Cleanup {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }

    fn snaps(&self) -> bool {
        false
    }

    /// Short: the bar it sits in is narrow at 1100 px, and the log line has the scope.
    fn prompt(&self) -> Prompt {
        let step = match &self.found {
            Some(found) => found.text(),
            None => "temizlenecek bir şey yok".to_owned(),
        };
        Prompt::new(LABEL, step).option("Temizle", "Enter")
    }

    fn point_count(&self) -> usize {
        0
    }

    /// Looks and says what it found; nothing to clean: says so and leaves.
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.scope(cx);
        let found = self.find(cx.doc, true);
        if found.is_empty() {
            cx.say(
                Level::Info,
                "Çizimi temizle: yinelenen ya da boş nesne, tekrarlanan köşe bulunamadı; çizim temiz.",
            );
            return Flow::Exit;
        }
        let scope = if self.whole {
            format!("bütün çizim, {} nesne", self.scope.len())
        } else {
            format!("seçili {} nesne", self.scope.len())
        };
        cx.say(
            Level::Info,
            format!(
                "Çizimi temizle: {}. Enter ile temizleyin ({scope}).",
                found.text()
            ),
        );
        self.found = Some(found);
        Flow::Stay
    }

    fn pointer_move(&mut self, _p: &crate::Pointer, _cx: &mut Context<'_>) {}

    /// A click does nothing: Enter, the button or a quick right click clean.
    fn pointer_down(&mut self, _p: &crate::Pointer, _cx: &mut Context<'_>) {}

    fn input(&mut self, _text: &str, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        self.clean(cx)
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn finished(&self) -> bool {
        self.done
    }

    fn preview(&self, _format: &Format) -> Preview {
        self.found
            .as_ref()
            .map(|f| f.drawn.clone())
            .unwrap_or_default()
    }
}
