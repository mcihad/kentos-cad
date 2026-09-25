//! Kapalı alan: the closed-area tool, the web's `PathTool` with
//! `closed: true` (`apps/web/src/tools/pathTool.ts`) on its
//! `PointInputTool` base (`drawTools.ts`), step for step:
//!
//! - points come from clicks (ortho and polar tracking applied) and from
//!   typed text (the shared grammar, `point_text`);
//! - Y turns the next segments into arcs (continuing tangentially, or shaped
//!   once by Açı, Merkez, Yarıçap, İkinci nokta, Doğrultu), D back to lines;
//!   in line mode U continues the last direction by a typed length; G takes
//!   the last point back;
//! - giving the first corner again closes and finishes the area: typed
//!   exactly, or clicked within the snap aperture once there are three
//!   corners; in arc mode the closing edge is the arc being drawn (ADR 0018);
//! - a confirm with three corners or more writes one polygon in one undo
//!   step; with fewer it warns, writes nothing and starts over.
//!
//! The web's messages are kept word for word. Every calculation is the
//! shared core's (`kentos-geometry-core`); none is written here.

use std::collections::BTreeMap;

use kentos_contracts::{Entity, EntityBase, PathEntity};
use kentos_geometry_core::geom::arc::DEFAULT_STEP;
use kentos_geometry_core::geom::bulge::{
    bulge_arc, bulge_of_sweep, bulge_path_outline, bulge_ring_area, bulge_through, has_bulges,
    segment_tangent, tangent_bulge,
};
use kentos_geometry_core::geometry::{bearing_grad, dist};
use kentos_geometry_core::jsmath::{PI, js_hypot};
use kentos_geometry_core::tools::drawing::{
    centre_bulge, offset_along, radial_point, radius_bulge, unit_toward,
};
use kentos_geometry_core::tools::point_input::{Tracking, constrain_cursor};
use kentos_geometry_core::tools::point_text::{js_trim, parse_number, point_from_text};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Pointer, Preview, Tag, Tool};

pub const ID: &str = "polygon";
pub const LABEL: &str = "Kapalı alan";
/// Corners a closed area needs.
const MIN: usize = 3;
/// How near the cursor must be to a polar ray to lock onto it, logical pixels (web `CAPTURE_PX`).
const CAPTURE_PX: f64 = 10.0;
/// Two points closer than this are the same point.
const SAME: f64 = 1e-9;

/// How the next arc segment is shaped (AutoCAD's PLINE arc options). The
/// default follows the path's end tangent; the others last one segment.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Spec {
    Tangent,
    /// The included angle in radians (typed in degrees), then the end point.
    Angle {
        sweep: Option<f64>,
    },
    /// The radius (typed), then the end point: the short arc, bending the way the path turns.
    Radius {
        r: Option<f64>,
    },
    /// The centre, then a point on the ray to the end (counter-clockwise).
    Centre {
        c: Option<Vec2>,
    },
    /// A point on the arc, then the end point.
    Second {
        via: Option<Vec2>,
    },
    /// The start direction, then the end point.
    Direction {
        dir: Option<Vec2>,
    },
}

/// The closed-area tool.
#[derive(Clone, Debug)]
pub struct Polygon {
    pts: Vec<Vec2>,
    /// One bulge per drawn segment (pts[i] → pts[i+1]).
    bulges: Vec<f64>,
    /// The effective cursor from the last pointer move.
    hover: Option<Vec2>,
    tracking: Option<Tracking>,
    arc_mode: bool,
    spec: Spec,
    /// A point on the first arc of a path, which has no tangent to follow.
    arc_via: Option<Vec2>,
    /// Line mode: waiting for a typed length along the last direction.
    ask_length: bool,
    /// Bulge of the closing segment: an arc when the area was closed on its first corner in arc mode.
    closing: f64,
    /// Where the button went down, while that click is being taken (closing on the first corner).
    pressed_at: Option<[f64; 2]>,
}

impl Default for Polygon {
    fn default() -> Self {
        Self::new()
    }
}

impl Polygon {
    pub fn new() -> Self {
        Self {
            pts: Vec::new(),
            bulges: Vec::new(),
            hover: None,
            tracking: None,
            arc_mode: false,
            spec: Spec::Tangent,
            arc_via: None,
            ask_length: false,
            closing: 0.0,
            pressed_at: None,
        }
    }

    fn last(&self) -> Option<Vec2> {
        self.pts.last().copied()
    }

    /// Travel direction at the last corner (the end tangent of the last segment).
    fn tangent(&self) -> Option<Vec2> {
        let n = self.pts.len();
        (n >= 2).then(|| {
            let bulge = self.bulges.get(n - 2).copied().unwrap_or(0.0);
            segment_tangent(self.pts[n - 2], self.pts[n - 1], bulge, true)
        })
    }

    /// Where a segment towards `p` really ends: the centre option puts it on the circle.
    fn end_for(&self, p: Vec2) -> Vec2 {
        match (self.spec, self.last()) {
            (Spec::Centre { c: Some(c) }, Some(last)) if self.arc_mode => {
                radial_point(c, dist(c, last), p).unwrap_or(p)
            }
            _ => p,
        }
    }

    /// The bulge the next segment to `p` would get; `None` when impossible
    /// or still waiting for a value.
    fn next_bulge(&self, p: Vec2) -> Option<f64> {
        let Some(last) = self.last() else {
            return Some(0.0);
        };
        if !self.arc_mode {
            return Some(0.0);
        }
        match self.spec {
            Spec::Angle { sweep } => sweep.map(bulge_of_sweep),
            // Bends the way the path turns towards p; counter-clockwise with no tangent yet.
            Spec::Radius { r } => r.and_then(|r| radius_bulge(last, p, r, self.tangent())),
            Spec::Centre { c } => c.and_then(|c| centre_bulge(c, last, self.end_for(p))),
            Spec::Second { via } => via.map(|via| bulge_through(last, via, p)),
            Spec::Direction { dir } => dir.and_then(|dir| tangent_bulge(last, dir, p)),
            Spec::Tangent => match self.tangent() {
                Some(t) => tangent_bulge(last, t, p),
                None => self.arc_via.map(|via| bulge_through(last, via, p)),
            },
        }
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        let line = format!("  {}", cx.format().point(p));
        cx.say(Level::Info, line);
        self.on_point(p, cx);
    }

    fn on_point(&mut self, p: Vec2, cx: &mut Context<'_>) {
        let Some(last) = self.last() else {
            self.pts.push(p);
            return;
        };
        // Options that take a point before the end point.
        if self.arc_mode {
            let tangent = self.tangent();
            match &mut self.spec {
                Spec::Centre { c: c @ None } => {
                    *c = Some(p);
                    return;
                }
                Spec::Second { via: via @ None } => {
                    *via = Some(p);
                    return;
                }
                Spec::Direction { dir: dir @ None } => {
                    if dist(last, p) > SAME {
                        *dir = unit_toward(last, p);
                    }
                    return;
                }
                Spec::Tangent if tangent.is_none() && self.arc_via.is_none() => {
                    self.arc_via = Some(p);
                    return;
                }
                _ => {}
            }
        }
        let end = self.end_for(p);
        if dist(last, end) <= SAME {
            return;
        }
        if self.closes_at(end, cx) {
            return self.close_on_first(cx);
        }
        let Some(bulge) = self.next_bulge(end) else {
            let text = match self.spec {
                Spec::Radius { r: Some(r) } => format!(
                    "Kiriş yarıçapın iki katından ({}) uzun; daha yakın bir nokta seçin.",
                    cx.format().length(2.0 * r)
                ),
                _ => "Bu nokta yayın tam arkasında kalıyor; başka bir nokta seçin.".to_owned(),
            };
            cx.say(Level::Warn, text);
            return;
        };
        self.pts.push(end);
        self.bulges.push(bulge);
        self.arc_via = None;
        // Arc options shape one segment; the path then continues tangentially.
        self.spec = Spec::Tangent;
    }

    /// A closed shape ends when its first corner is given again (ADR 0018):
    /// typed exactly, or clicked within the snap aperture once there are
    /// three corners. The first corner is never written twice.
    fn closes_at(&self, end: Vec2, cx: &Context<'_>) -> bool {
        let Some(&first) = self.pts.first() else {
            return false;
        };
        if dist(first, end) <= SAME {
            return true;
        }
        let Some(pressed) = self.pressed_at else {
            return false;
        };
        if self.pts.len() < MIN {
            return false;
        }
        let s = cx.view.to_screen(first);
        js_hypot(s[0] - pressed[0], s[1] - pressed[1]) <= cx.draft.snap_aperture
    }

    fn close_on_first(&mut self, cx: &mut Context<'_>) {
        if self.pts.len() < MIN {
            cx.say(
                Level::Warn,
                format!("{LABEL} için en az 3 köşe gerekir; ilk köşe ikinci kez eklenmedi."),
            );
            return;
        }
        // In arc mode the segment back to the first corner is the arc being drawn.
        let Some(bulge) = self.next_bulge(self.pts[0]) else {
            cx.say(
                Level::Warn,
                "İlk köşe yayın tam arkasında kalıyor; alanı Enter ile düz kenarla kapatın.",
            );
            return;
        };
        self.closing = bulge;
        self.finish(cx);
    }

    /// Option letters: Y, D, U, G and the arc options. False when the key is none of them now.
    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
        let arc = match key {
            "A" => Some(Spec::Angle { sweep: None }),
            "M" => Some(Spec::Centre { c: None }),
            "R" => Some(Spec::Radius { r: None }),
            "İ" | "I" => Some(Spec::Second { via: None }),
            "T" => Some(Spec::Direction { dir: None }),
            _ => None,
        };
        if key == "Y" || key == "D" {
            self.arc_mode = key == "Y";
            self.arc_via = None;
            self.spec = Spec::Tangent;
            self.ask_length = false;
        } else if let (true, Some(arc), false) = (self.arc_mode, arc, self.pts.is_empty()) {
            self.spec = arc;
        } else if !self.arc_mode && key == "U" && !self.pts.is_empty() {
            if self.tangent().is_none() {
                cx.say(
                    Level::Warn,
                    "Uzunlukla devam için önce bir parça çizin; ilk parçanın doğrultusu yok.",
                );
                return true;
            }
            self.ask_length = true;
        } else if key == "G" && self.arc_via.is_some() {
            self.arc_via = None;
        } else if key == "G" && !self.pts.is_empty() {
            self.pts.pop();
            self.bulges.pop();
            self.spec = Spec::Tangent;
        } else {
            return false;
        }
        true
    }

    /// Commits the area when it has enough corners, then starts over.
    fn finish(&mut self, cx: &mut Context<'_>) {
        if self.pts.len() < MIN {
            cx.say(
                Level::Warn,
                format!("{LABEL} için en az {MIN} nokta gerekir."),
            );
            self.reset();
            return;
        }
        let pts = self.pts.clone();
        let bulges = self.full_bulges();
        let area = bulge_ring_area(&pts, bulges.as_deref()).abs();
        if self.create(&pts, bulges, cx) {
            let text = format!("Kapalı alan eklendi: {}", cx.format().area(area));
            cx.say(Level::Success, text);
        }
        self.reset();
    }

    /// Bulges of the finished shape: it closes straight unless it was closed
    /// on its first corner with an arc; none when every edge is straight.
    fn full_bulges(&self) -> Option<Vec<f64>> {
        let mut all = self.bulges.clone();
        all.push(self.closing);
        has_bulges(Some(&all)).then_some(all)
    }

    /// Writes the polygon on the active layer as one undo step. False (with a
    /// message) when the layer is locked; a hidden layer is written with a warning.
    fn create(&self, pts: &[Vec2], bulges: Option<Vec<f64>>, cx: &mut Context<'_>) -> bool {
        let layers = cx.doc.layers();
        let id = layers.active().to_owned();
        let Some(node) = layers.get(&id) else {
            return false;
        };
        let name = node.name.clone();
        let locked = layers.is_locked(&id);
        let hidden = !layers.is_visible(&id);
        if locked {
            cx.say(
                Level::Warn,
                format!(
                    "“{name}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin."
                ),
            );
            return false;
        }
        if hidden {
            cx.say(
                Level::Warn,
                format!("“{name}” katmanı gizli; çizilen nesne görünmeyecek."),
            );
        }
        let entity = Entity::Polygon(PathEntity {
            base: EntityBase {
                id: 0,
                layer_id: id,
                color: None,
                attrs: BTreeMap::new(),
                label: None,
                symbol: None,
            },
            pts: pts
                .iter()
                .map(|p| kentos_contracts::Vec2 { x: p.x, y: p.y })
                .collect(),
            bulges,
            holes: None,
        });
        match cx.doc.add(entity) {
            Ok(_) => true,
            Err(full) => {
                cx.say(Level::Error, full.to_string());
                false
            }
        }
    }

    fn reset(&mut self) {
        self.pts.clear();
        self.bulges.clear();
        self.arc_via = None;
        self.arc_mode = false;
        self.spec = Spec::Tangent;
        self.ask_length = false;
        self.closing = 0.0;
    }

    /// The effective cursor for the next point: ortho (Shift turns it over)
    /// and polar tracking from the last point, by the shared core.
    fn constrain(&mut self, p: &Pointer, cx: &Context<'_>) -> Vec2 {
        let Some(from) = self.last() else {
            self.tracking = None;
            return p.world;
        };
        let c = constrain_cursor(
            Some(from),
            p.world,
            false,
            cx.draft.ortho != p.shift,
            cx.draft.polar,
            cx.view.world_length(CAPTURE_PX),
        );
        self.tracking = c.tracking;
        c.point
    }

    fn arc_options(prompt: Prompt, done: bool) -> Prompt {
        let prompt = prompt
            .option("Düz", "D")
            .option("Açı", "A")
            .option("Merkez", "M")
            .option("Yarıçap", "R")
            .option("İkinci nokta", "İ")
            .option("Doğrultu", "T")
            .option("Geri", "G");
        if done {
            prompt.option("Bitir", "Enter")
        } else {
            prompt
        }
    }
}

impl Tool for Polygon {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let n = self.pts.len();
        if n == 0 {
            return Prompt::new(LABEL, "ilk noktayı belirtin");
        }
        let done = n >= MIN;
        if self.ask_length {
            return Prompt::new(LABEL, "son doğrultuda devam edilecek uzunluğu yazın");
        }
        if !self.arc_mode {
            let prompt = Prompt::new(LABEL, "sonraki noktayı belirtin")
                .option("Yay", "Y")
                .option("Uzunluk", "U")
                .option("Geri", "G");
            return if done {
                prompt.option("Bitir", "Enter")
            } else {
                prompt
            };
        }
        let step = match self.spec {
            Spec::Angle { sweep: None } => {
                return Prompt::new(
                    LABEL,
                    "yayın iç açısını derece olarak yazın (artı saat yönünün tersine)",
                );
            }
            Spec::Radius { r: None } => return Prompt::new(LABEL, "yayın yarıçapını yazın"),
            Spec::Angle { .. } | Spec::Radius { .. } => "yayın bitiş noktasını belirtin",
            Spec::Centre { c: Some(_) } => "yayın bitiş doğrultusunu gösterin",
            Spec::Centre { c: None } => "yayın merkezini gösterin",
            Spec::Second { via: Some(_) } | Spec::Direction { dir: Some(_) } => {
                "yayın bitiş noktasını belirtin"
            }
            Spec::Second { via: None } => "yayın üzerinden geçeceği bir nokta belirtin",
            Spec::Direction { dir: None } => "yayın başlangıç doğrultusunu gösterin",
            Spec::Tangent if self.tangent().is_none() && self.arc_via.is_none() => {
                "yayın üzerinden geçeceği bir nokta belirtin"
            }
            Spec::Tangent => "yayın bitiş noktasını belirtin",
        };
        Self::arc_options(Prompt::new(LABEL, step), done)
    }

    fn point_count(&self) -> usize {
        self.pts.len()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(self.constrain(p, cx));
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.pressed_at = Some(p.screen);
        let point = self.constrain(p, cx);
        self.accept(point, cx);
        self.pressed_at = None;
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.option(&upper_tr(js_trim(text)), cx) {
            return true;
        }
        let number = parse_number(text);
        let plain = number.filter(|_| !text.contains([',', ';', '@', '<']));
        if let Some(n) = plain {
            if self.ask_length {
                match (self.tangent(), self.last()) {
                    (Some(t), Some(last)) if n > 0.0 => {
                        self.ask_length = false;
                        self.accept(offset_along(last, t, n), cx);
                    }
                    _ => cx.say(Level::Warn, "Uzunluk sıfırdan büyük olmalı."),
                }
                return true;
            }
            if self.arc_mode
                && let Spec::Angle { sweep: None } = self.spec
            {
                if n.abs() < 1e-9 || n.abs() >= 360.0 {
                    cx.say(Level::Warn, "İç açı 0 ile ±360 derece arasında olmalı.");
                } else {
                    self.spec = Spec::Angle {
                        sweep: Some((n * PI) / 180.0),
                    };
                }
                return true;
            }
            if self.arc_mode
                && let Spec::Radius { r: None } = self.spec
            {
                if n > 0.0 {
                    self.spec = Spec::Radius { r: Some(n) };
                } else {
                    cx.say(Level::Warn, "Yarıçap sıfırdan büyük olmalı.");
                }
                return true;
            }
        }
        // Object tracking has no line on the desktop yet: a bare number follows the cursor.
        match point_from_text(text, self.last(), self.hover, |_| None) {
            Some(p) => {
                self.accept(p, cx);
                true
            }
            None => false,
        }
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.pts.is_empty() {
            return Flow::Exit;
        }
        self.finish(cx);
        Flow::Stay
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        // Newest first: the tool's own Geri (G), which takes back the last
        // point (or the arc's through-point); with no point, the drawing's undo.
        !self.pts.is_empty() && self.option("G", cx)
    }

    fn preview(&self, format: &Format) -> Preview {
        let mut pts = self.pts.clone();
        let mut bulges = self.bulges.clone();
        let end = self.hover.map(|h| self.end_for(h));
        let hb = end.and_then(|e| self.next_bulge(e));
        if let (Some(end), Some(hb)) = (end, hb) {
            pts.push(end);
            bulges.push(hb);
        }
        bulges.push(0.0);
        let ring =
            (pts.len() >= MIN).then(|| bulge_path_outline(&pts, Some(&bulges), true, DEFAULT_STEP));
        let path = bulge_path_outline(&pts, Some(&bulges), false, DEFAULT_STEP);
        let last = self.last();
        let mut guides = Vec::new();
        // Helper lines for points given before the end point.
        let guide = match self.spec {
            Spec::Second { via } => via,
            Spec::Centre { c } => c,
            _ => self.arc_via,
        };
        match (guide, last, self.hover) {
            (Some(guide), Some(last), _) => guides.push([last, guide]),
            (None, Some(last), Some(hover)) if self.arc_mode && hb.is_none() => {
                guides.push([last, hover]);
            }
            _ => {}
        }
        if let (Spec::Centre { c: Some(c) }, Some(hover)) = (self.spec, self.hover) {
            guides.push([c, hover]);
        }
        let tag = match (self.hover, last, end) {
            (Some(hover), Some(last), Some(end)) => {
                let arc = hb.and_then(|b| bulge_arc(last, end, b));
                let mut lines = match arc {
                    Some(a) => vec![
                        format!("Yay r {}", format.length(a.r)),
                        format!("Yay boyu {}", format.length(a.r * a.sweep.abs())),
                    ],
                    None => vec![
                        format.length(dist(last, end)),
                        format!("Semt {}", format.bearing(bearing_grad(last, end))),
                    ],
                };
                if pts.len() >= MIN {
                    let area = bulge_ring_area(&pts, Some(&bulges)).abs();
                    lines.push(format!("Alan {}", format.area(area)));
                }
                Some(Tag { at: hover, lines })
            }
            _ => None,
        };
        Preview {
            path,
            ring,
            guides,
            tracking: tag.as_ref().and(self.tracking),
            tag,
        }
    }
}
