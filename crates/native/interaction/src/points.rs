//! What the point-driven tools share: the web's `PointInputTool` base
//! (`apps/web/src/tools/drawTools.ts`) without its DOM side. The effective
//! cursor (ortho, polar tracking), the echo of a taken point, and how a
//! product command's answer reaches the user.

use kentos_commands::CommandResult;
use kentos_geometry_core::tools::locks::{Direction, constrain_locked, perpendicular};
use kentos_geometry_core::tools::point_input::Tracking;
use kentos_geometry_core::tools::ucs::constrain_cursor_in;
use kentos_native_application::codes;

use crate::Vec2;
use crate::log::Level;
use crate::tool::{Context, Pointer};

/// How near the cursor must be to a polar ray to lock onto it, logical pixels (web `CAPTURE_PX`).
const CAPTURE_PX: f64 = 10.0;
/// Two points closer than this are the same point.
pub(crate) const SAME: f64 = 1e-9;

/// What rules the next point's place (docs/adr/0268 §1), first to last: a locked coordinate, a locked direction
/// (Açı, Sapma, Paralel, Dik), Dik açı, an object snap's or object tracking's point (exact), ortho (Shift
/// turning it over while held), polar tracking (onto its ray), else the pointer itself. A locked length holds
/// with each of them; ortho and polar tracking are never both: ortho comes first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aid {
    /// No point yet to measure from, or nothing holds the pointer.
    Free,
    Coordinates,
    Direction,
    RightAngle,
    Snap,
    Tracking,
    Ortho,
    Polar,
}

impl Aid {
    /// Its name in the traces (`aid`).
    pub fn key(self) -> &'static str {
        match self {
            Aid::Free => "free",
            Aid::Coordinates => "coordinates",
            Aid::Direction => "direction",
            Aid::RightAngle => "rightAngle",
            Aid::Snap => "snap",
            Aid::Tracking => "tracking",
            Aid::Ortho => "ortho",
            Aid::Polar => "polar",
        }
    }
}

/// Whether ortho rules the pointer now: the setting, turned over while Shift is held (docs/adr/0268 §2).
pub fn ortho_now(ortho: bool, shift: bool) -> bool {
    ortho != shift
}

/// The effective cursor for the next point after `from`: ortho (Shift turns
/// it over) and polar tracking, by the shared core; with the tracking ray it
/// locked onto. The pointer's own point when there is no point yet, and an
/// object snap's point exactly: ortho and polar never move it (the web's
/// `constrainPoint`, docs/adr/0029). The digitizing locks hold it
/// (docs/adr/0166 §2): a locked direction projects even a snapped point; a
/// length alone with the cursor on `from` leaves it there.
pub(crate) fn constrain(
    from: Option<Vec2>,
    p: &Pointer,
    cx: &Context<'_>,
) -> (Vec2, Option<Tracking>) {
    let (point, tracking, _) = constrain_ruled(from, p, cx);
    (point, tracking)
}

/// [`constrain`] and what ruled the point (docs/adr/0268 §1): the one rule both the tools and the status bar read.
pub(crate) fn constrain_ruled(
    from: Option<Vec2>,
    p: &Pointer,
    cx: &Context<'_>,
) -> (Vec2, Option<Tracking>, Aid) {
    // A reference point stands in for the last corner (docs/adr/0166 §5).
    let Some(from) = cx.locks.reference.or(from) else {
        return (p.world, None, Aid::Free);
    };
    let exact = p.snap.is_some() || p.tracked;
    let ortho = ortho_now(cx.draft.ortho, p.shift);
    // An exact point, ortho or polar tracking: what rules a point no lock holds.
    let unlocked = |tracking: Option<Tracking>| {
        if p.snap.is_some() {
            Aid::Snap
        } else if p.tracked {
            Aid::Tracking
        } else if ortho {
            Aid::Ortho
        } else if tracking.is_some() {
            Aid::Polar
        } else {
            Aid::Free
        }
    };
    let tol = cx.view.world_length(CAPTURE_PX);
    // Dik açı (docs/adr/0166 §4): from the second edge on, square to the one
    // before, either way; a direction locked by hand comes first, a length
    // lock holds with it.
    let square = cx
        .locks
        .travel
        .filter(|_| cx.draft.right_angle && cx.locks.toward.is_none())
        .map(|t| Direction {
            u: perpendicular(t),
            both: true,
        });
    // The drawing's UCS (docs/adr/0261 §4): ortho, polar tracking and a typed angle follow its axes.
    let frame = cx.frame();
    // Coordinate locks (docs/adr/0267 §1): the point in the UCS, ortho and polar tracking left out.
    if cx.locks.coordinates() {
        let mut held = cx.locks.clone();
        held.frame = frame;
        return match held.point(from, p.world, cx.format().angles()) {
            Some(q) => (q, None, Aid::Coordinates),
            None => (from, None, Aid::Coordinates),
        };
    }
    if cx.locks.any() || square.is_some() {
        let mut held = cx.locks.clone();
        // A typed angle is from the UCS's axes (the lock's frame); the other directions are the objects', in
        // the world.
        held.frame = frame;
        let mut locks = held.locks(cx.format().angles());
        let aid = if locks.direction.is_some() {
            Aid::Direction
        } else {
            Aid::RightAngle
        };
        locks.direction = locks.direction.or(square);
        if locks.direction.is_none() {
            // A length alone: the cursor ruled in the UCS first, then held at the length.
            let free = constrain_cursor_in(
                &frame,
                Some(from),
                p.world,
                exact,
                ortho,
                cx.draft.polar,
                tol,
            );
            let aid = unlocked(free.tracking);
            return match constrain_locked(Some(from), free.point, true, false, None, tol, locks) {
                Some(c) => (c.point, free.tracking, aid),
                None => (from, None, aid),
            };
        }
        return match constrain_locked(
            Some(from),
            p.world,
            exact,
            ortho,
            cx.draft.polar,
            tol,
            locks,
        ) {
            Some(c) => (c.point, c.tracking, aid),
            None => (from, None, aid),
        };
    }
    let c = constrain_cursor_in(
        &frame,
        Some(from),
        p.world,
        exact,
        ortho,
        cx.draft.polar,
        tol,
    );
    (c.point, c.tracking, unlocked(c.tracking))
}

/// A taken point, said as the web says it: `  Y 487012.000  X 4420000.000`.
pub(crate) fn echo(p: Vec2, cx: &mut Context<'_>) {
    let line = format!("  {}", cx.format().point(p));
    cx.say(Level::Info, line);
}

/// A product command's answer, as the tool reports it (docs/adr/0022,
/// 0027): the warnings of a completed write, then its output; a refusal's
/// message and `None`. A document out of slots is not the user's to fix
/// here, so it is an error; the rest are warnings, word for word the
/// command's (the locked and hidden layer texts are the tools' own).
pub(crate) fn written<T>(result: CommandResult<T>, cx: &mut Context<'_>) -> Option<T> {
    match result {
        CommandResult::Completed { output, warnings } => {
            for warning in warnings {
                // What Çakışma and Topoloji did is news, not a warning (docs/adr/0162).
                let level = match warning.code.as_str() {
                    codes::AREA_CLIPPED | codes::CORNERS_JOINED => Level::Info,
                    _ => Level::Warn,
                };
                cx.say(level, warning.message);
            }
            Some(output)
        }
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => {
            let level = if error.code == codes::SLOTS_EXHAUSTED {
                Level::Error
            } else {
                Level::Warn
            };
            cx.say(level, error.message);
            None
        }
        CommandResult::Queued { .. } | CommandResult::Cancelled => None,
    }
}

/// `Vec2` as the contracts carry it.
pub(crate) fn wire(p: Vec2) -> kentos_commands::Vec2 {
    kentos_commands::Vec2 { x: p.x, y: p.y }
}

/// A shape command's writer: the object it wrote, by its slot.
pub(crate) type Writer<'w> = &'w mut dyn FnMut(&mut Context<'_>) -> Option<u32>;

/// The session's Çakışma and Topoloji as a command's rules for a new area going on `layer` (docs/adr/0162,
/// 0622 §9): the layers it must not overlap, whether it shares corners; none when they change nothing.
pub(crate) fn area_rules(cx: &mut Context<'_>, layer: &str) -> Option<kentos_commands::AreaRules> {
    let avoid = crate::overlap::overlap_layers(cx, layer);
    if avoid.is_empty() && cx.draft.overlap == crate::tool::Overlap::Layers {
        cx.say(
            Level::Warn,
            "Seçili katmanlarda önle kipinde seçili katman yok: alan olduğu gibi yazıldı. Katmanları Çakışma hücresinin menüsünden seçin.",
        );
    }
    let rules = kentos_commands::AreaRules {
        avoid,
        topology: cx.draft.topology,
        points: cx.draft.topology && cx.draft.topology_points,
    };
    (!rules.is_empty()).then_some(rules)
}

/// The net area of the object in `slot`: its parts less their holes.
pub(crate) fn area_of_slot(cx: &Context<'_>, slot: u32) -> f64 {
    use kentos_geometry_core::geom::region::net_area;
    use kentos_geometry_core::ops::areas::areas_of_entity;
    cx.doc
        .get(kentos_domain::Slot(slot))
        .map(|e| {
            areas_of_entity(&kentos_native_application::geometry::shape(e))
                .iter()
                .map(net_area)
                .sum()
        })
        .unwrap_or(0.0)
}

/// A new area a tool built by its outline (a rectangle, a regular polygon, a sector, a cloud), written by its own
/// command (docs/adr/0290, 0622 §9) with the session's Çakışma and Topoloji in its input ([`area_rules`]): the
/// command cuts away what overlaps and shares corners with the neighbours in its own step, and says what it did.
/// The area written, or `None` when the command refused (its reason said).
pub(crate) fn write_area_by(
    d: &mut Taken,
    cx: &mut Context<'_>,
    writer: Writer<'_>,
) -> Option<f64> {
    let id = writer(cx)?;
    d.note(id, cx);
    Some(area_of_slot(cx, id))
}

/// Objects a tool built (an ellipse, a spline, a perpendicular …) written
/// through the product command `cad.entities.create` (docs/adr/0057): the
/// active layer and the current colour explicit in its input (CMD-07; the
/// web's `writeObjects`). One undo step, “Ekle” or the tool's `operation`.
/// The command's answer, or `None` when it refused (its reason said).
pub(crate) fn write_objects(
    geometries: Vec<kentos_commands::EntityGeometry>,
    operation: Option<kentos_commands::CreateOperation>,
    cx: &mut Context<'_>,
) -> Option<kentos_commands::EntitiesCreated> {
    write_objects_with(geometries, None, operation, cx)
}

/// `write_objects` with the objects' attributes (Blok ekle's values, docs/adr/0144 §7),
/// over an object template's.
pub(crate) fn write_objects_with(
    geometries: Vec<kentos_commands::EntityGeometry>,
    attrs: Option<std::collections::BTreeMap<String, String>>,
    operation: Option<kentos_commands::CreateOperation>,
    cx: &mut Context<'_>,
) -> Option<kentos_commands::EntitiesCreated> {
    let each = geometries.into_iter().map(|g| (g, attrs.clone())).collect();
    write_objects_each(each, operation, cx)
}

/// `write_objects` with each object's own attributes (Km yaz's `Km`,
/// docs/adr/0189 §3), over an object template's.
pub(crate) fn write_objects_each(
    objects: Vec<(
        kentos_commands::EntityGeometry,
        Option<std::collections::BTreeMap<String, String>>,
    )>,
    operation: Option<kentos_commands::CreateOperation>,
    cx: &mut Context<'_>,
) -> Option<kentos_commands::EntitiesCreated> {
    use kentos_native_application::{ExecutionContext, create};
    let input = kentos_commands::EntitiesCreate {
        layer_id: cx.doc.layers().active().to_owned(),
        objects: objects
            .into_iter()
            .map(|(geometry, attrs)| kentos_commands::NewObject {
                // And the current line type (docs/adr/0260 §2).
                line_type: geometry
                    .draws_lines()
                    .then_some(cx.draft.line_type)
                    .flatten(),
                // The current weight goes to what is drawn with lines (docs/adr/0139).
                line_weight: geometry
                    .draws_lines()
                    .then_some(cx.draft.line_weight)
                    .flatten(),
                geometry,
                color: cx.draft.color_text(),
                // An object template's symbol, attributes and label (docs/adr/0176 §3).
                attrs: cx.template_attrs(attrs),
                label: cx.template_label(),
                label_of: None,
                label_scale: None,
                symbol: cx.template_symbol(),
                order: None,
                transparency: None,
            })
            .collect(),
        operation,
        expected_revision: None,
    };
    let result = create::execute(&mut ExecutionContext::new(cx.doc), input);
    written(result, cx)
}

/// A construction tool's objects written through `cad.constructions.create` (docs/adr/0622 §9): the active layer, the
/// current colour, weight and line type, and an object template's attributes, label and symbol, as
/// [`write_objects`] gives them; the command computes the objects and names the undo step after the tool.
pub(crate) fn write_construction(
    construction: kentos_commands::Construction,
    cx: &mut Context<'_>,
) -> Option<kentos_commands::EntitiesCreated> {
    use kentos_native_application::{ExecutionContext, constructions};
    let input = construction_input(construction, cx);
    let result = constructions::execute(&mut ExecutionContext::new(cx.doc), input);
    written(result, cx)
}

/// The command's input for a construction, as [`write_construction`] sends it (a tool's preview asks with it too).
pub(crate) fn construction_input(
    construction: kentos_commands::Construction,
    cx: &Context<'_>,
) -> kentos_commands::ConstructionsCreate {
    kentos_commands::ConstructionsCreate {
        layer_id: cx.doc.layers().active().to_owned(),
        construction,
        color: cx.draft.color_text(),
        line_weight: cx.draft.line_weight,
        line_type: cx.draft.line_type,
        attrs: cx.template_attrs(None),
        label: cx.template_label(),
        symbol: cx.template_symbol(),
        expected_revision: None,
    }
}

/// Points as the contracts carry them.
pub(crate) fn wire_all(pts: &[Vec2]) -> Vec<kentos_commands::Vec2> {
    pts.iter().map(|p| wire(*p)).collect()
}

/// How far a preview's unbounded line reaches either way: `times` the
/// diagonal of what the view shows (the web's `strokeInfinite` and
/// `drawRef`: long enough to cross any view).
pub(crate) fn reach(view: &dyn crate::tool::View, times: f64) -> f64 {
    let b = view.visible();
    kentos_geometry_core::jsmath::js_hypot(b.max_x - b.min_x, b.max_y - b.min_y) * times
}

/// The web's `PointInputTool.draw`: the points so far and the cursor as one
/// solid line, the length and bearing of its last segment beside the cursor,
/// and the polar ray the cursor is locked to.
pub(crate) fn chain_preview(d: &Taken, format: &crate::format::Format) -> crate::tool::Preview {
    use kentos_geometry_core::geometry::{bearing_grad, dist};
    let path: Vec<Vec2> = d.pts.iter().copied().chain(d.hover).collect();
    let tag = match (d.last(), d.hover) {
        (Some(last), Some(hover)) => Some(crate::tool::Tag {
            at: hover,
            lines: vec![
                format.length(dist(last, hover)),
                format!("Semt {}", format.bearing(bearing_grad(last, hover))),
            ],
        }),
        _ => None,
    };
    crate::tool::Preview {
        path,
        tracking: tag.as_ref().and(d.tracking),
        tag,
        ..crate::tool::Preview::default()
    }
}

/// Typed text that is a number and nothing else: no separator, relative or
/// polar mark (the web's `!/[,;@<]/.test(text)` beside `parseNumber`).
pub(crate) fn plain_number(text: &str) -> Option<f64> {
    if text.contains([',', ';', '@', '<']) {
        return None;
    }
    kentos_geometry_core::tools::point_text::parse_number(text)
}

/// A plain number that is a length (a distance, a radius, a tolerance, an
/// elevation), typed in the project's unit, in metres (docs/adr/0165 §2;
/// the web's `parseLength`); with its own unit (`2.5m`, `5'6"`) or as a sum
/// in brackets in any project (docs/adr/0262 §3, 0266).
pub(crate) fn plain_length(text: &str, cx: &crate::tool::Context<'_>) -> Option<f64> {
    plain_value(text, cx).map(|(_, m)| m)
}

/// A plain typed value both ways: the number in the project's unit (for an
/// angle, a count or a sign) and the length it is in metres. A length with its
/// own unit gives the number its metres are in the project's unit
/// (docs/adr/0266 §1); none for a point's text.
pub(crate) fn plain_value(text: &str, cx: &crate::tool::Context<'_>) -> Option<(f64, f64)> {
    if text.contains([',', ';', '@', '<']) && !text.trim_start().starts_with('(') {
        return None;
    }
    let f = cx.format();
    match kentos_geometry_core::tools::point_text::parse_length(text)? {
        kentos_geometry_core::tools::point_text::Len::Unit(n) => Some((n, f.to_metres(n))),
        other => {
            let m = other.metres(|n| f.to_metres(n));
            Some((f.from_metres(m), m))
        }
    }
}

/// What every point-input tool keeps (the web's `PointInputTool` fields,
/// docs/adr/0032): its points, the effective cursor, and the objects written
/// for the object being drawn, so Ctrl+Z can take the newest back as an
/// undo while the drawing has not changed since.
#[derive(Clone, Debug, Default)]
pub(crate) struct Taken {
    pub pts: Vec<Vec2>,
    /// The effective cursor from the last pointer move.
    pub hover: Option<Vec2>,
    pub tracking: Option<Tracking>,
    made: Vec<kentos_domain::Slot>,
    made_at: Option<u64>,
}

impl Taken {
    pub fn last(&self) -> Option<Vec2> {
        self.pts.last().copied()
    }

    /// The effective cursor for the pointer, from the last point (ortho,
    /// polar tracking; a snapped point exact): the web's `constrain`.
    pub fn constrain(&mut self, p: &Pointer, cx: &Context<'_>) -> Vec2 {
        let (point, tracking) = constrain(self.last(), p, cx);
        self.tracking = tracking;
        point
    }

    /// The web's `accept` before the tool's `onPoint`: the point is echoed,
    /// and a first point starts a new object (what was written before is the
    /// drawing's to undo).
    pub fn begin(&mut self, p: Vec2, cx: &mut Context<'_>) {
        echo(p, cx);
        if self.pts.is_empty() {
            self.made.clear();
        }
    }

    /// An object written for the object being drawn (the web's `noteMade`).
    pub fn note(&mut self, slot: u32, cx: &Context<'_>) {
        self.made.push(kentos_domain::Slot(slot));
        self.made_at = Some(cx.doc.revision());
    }

    /// Takes back the newest object written for the object being drawn, as
    /// an undo, when the drawing has not changed since (the web's `undoLastMade`).
    pub fn undo_last_made(&mut self, cx: &mut Context<'_>) -> bool {
        if self.made.is_empty() || self.made_at != Some(cx.doc.revision()) {
            return false;
        }
        self.made.pop();
        cx.doc.undo();
        self.made_at = Some(cx.doc.revision());
        true
    }

    /// Drops the draft's points and what it wrote (the web's `reset`).
    pub fn reset(&mut self) {
        self.pts.clear();
        self.made.clear();
    }

    /// Ctrl+Z for a tool without Geri (G) (the web's `undoStep`): the object
    /// written for the draft (as an undo), else the draft's last point when
    /// its step follows from its points alone, else the draft starts over.
    /// False when there was nothing: the drawing is undone then.
    pub fn undo_step(&mut self, steps_from_points: bool, cx: &mut Context<'_>) -> bool {
        if self.undo_last_made(cx) {
            return true;
        }
        if self.pts.is_empty() {
            return false;
        }
        if steps_from_points {
            self.pts.pop();
        } else {
            self.reset();
        }
        true
    }
}
