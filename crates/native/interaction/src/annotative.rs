//! Ölçekli açıklamalar from the interface (docs/adr/0323 §4): Öznitelikler's Ölçekli, Öbür ölçekler and Ayrı değerler, the
//! ribbon's Ölçekli yap and Ölçeği sıfırla on the selection, and the tool Ölçekteki yeri. Each writes by
//! `cad.annotations.scales`; Öbür ölçekler's add and remove are one undo step.
//!
//! **Ölçekteki yeri**: a click on an annotative text, dimension or leader, then where it goes at one of its other
//! scales (Ölçek (Ö) chooses which; the first by default): a text's point, a leader's landing start, a dimension's
//! line through the click. The preview is the representation at that scale. Enter, a quick right click or Esc ends;
//! Esc first lets the object go.

use kentos_contracts::{
    AnnotationsScales, AnnotativeOperation, AtScale, Entity, annotative_of, at_scale, same_scale,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::geom::dimension::{DimensionGeom, dimension_offset_at, layout_dimension};
use kentos_native_application::{ExecutionContext, annotations_scales};

use crate::Vec2;
use crate::edge;
use crate::format::Format;
use crate::js_trim;
use crate::log::Level;
use crate::points::wire;
use crate::prompt::Prompt;
use crate::properties::said;
use crate::tool::{Context, Cursor, Flow, OptionChoice, Pointer, Preview, Stroke, Tag, Tool};

/// Ölçekteki yeri's id: its command is `tool.annotativePlace`.
pub const PLACE_ID: &str = "annotativePlace";

/// A scale as the interface writes it: 1:100, 1:12.5.
pub fn ratio(scale: f64) -> String {
    let text = format!("{scale:.6}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    format!("1:{text}")
}

/// An object's other scales as Öznitelikler's Öbür ölçekler says them: `1:50*, 1:200` (a scale that sets something
/// apart with an asterisk), or `Yok`; none when it is not annotative.
pub fn others_text(e: &Entity) -> Option<String> {
    let a = annotative_of(e)?;
    if a.scales.is_empty() {
        return Some("Yok".to_owned());
    }
    let parts: Vec<String> = a
        .scales
        .iter()
        .map(|s| {
            if s.is_plain() {
                ratio(s.scale)
            } else {
                format!("{}*", ratio(s.scale))
            }
        })
        .collect();
    Some(parts.join(", "))
}

/// The other scales typed in Öznitelikler's Öbür ölçekler: `1:50, 200; 1:500` (`Yok` or nothing: none); `Err` with
/// what to say.
pub fn parse_scales(text: &str) -> Result<Vec<f64>, String> {
    let mut out: Vec<f64> = Vec::new();
    for part in text
        .split([',', ';', ' '])
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        if part == "*" || part.eq_ignore_ascii_case("yok") {
            continue;
        }
        let number = part
            .strip_prefix("1:")
            .unwrap_or(part)
            .trim_end_matches('*');
        let Ok(s) = number.parse::<f64>() else {
            return Err(format!(
                "“{part}” bir ölçek değil: 1:50 ya da 50 gibi yazın, virgülle ayırın."
            ));
        };
        if !out.iter().any(|x| same_scale(*x, s)) {
            out.push(s);
        }
    }
    Ok(out)
}

/// The persistent ids of the objects at `slots` (all of them; the command says which it leaves out).
fn uids(doc: &Document, slots: &[Slot]) -> Vec<String> {
    slots
        .iter()
        .filter_map(|s| doc.uid(*s).map(|u| u.to_string()))
        .collect()
}

/// One `cad.annotations.scales` of `operation` on the objects at `slots`; what to say.
pub fn run(
    doc: &mut Document,
    slots: &[Slot],
    operation: AnnotativeOperation,
    scales: Vec<f64>,
) -> Vec<String> {
    let input = AnnotationsScales {
        operation,
        uids: uids(doc, slots),
        scales,
        scale: None,
        point: None,
        rotation: None,
        offset: None,
        text_shift: None,
        expected_revision: None,
    };
    said(annotations_scales::execute(
        &mut ExecutionContext::new(doc),
        input,
    ))
}

/// Öznitelikler's Ölçekler (docs/adr/0323 §4): the objects at `slots` take part in `wanted` besides their own scale,
/// and in no other; one undo step.
pub fn set_scales(doc: &mut Document, slots: &[Slot], wanted: &[f64]) -> Vec<String> {
    let mut removed: Vec<f64> = Vec::new();
    for slot in slots {
        let Some(a) = doc.get(*slot).and_then(annotative_of) else {
            continue;
        };
        for s in &a.scales {
            if !wanted.iter().any(|w| same_scale(*w, s.scale))
                && !removed.iter().any(|r| same_scale(*r, s.scale))
            {
                removed.push(s.scale);
            }
        }
    }
    let added: Vec<f64> = wanted
        .iter()
        .copied()
        .filter(|w| {
            slots.iter().any(|slot| {
                doc.get(*slot)
                    .and_then(annotative_of)
                    .is_some_and(|a| !a.takes(*w))
            })
        })
        .collect();
    if removed.is_empty() && added.is_empty() {
        return Vec::new();
    }
    let group = doc.begin_group(annotations_scales::label(AnnotativeOperation::Add));
    let mut out = Vec::new();
    if !removed.is_empty() {
        out.extend(run(doc, slots, AnnotativeOperation::Remove, removed));
    }
    if !added.is_empty() {
        out.extend(run(doc, slots, AnnotativeOperation::Add, added));
    }
    doc.end_group(group);
    out
}

/// An annotative object a click may pick: on an unlocked layer, with another scale.
fn placeable(e: &Entity, doc: &Document) -> bool {
    edge::unlocked(e, doc) && annotative_of(e).is_some_and(|a| !a.scales.is_empty())
}

/// The annotative object a click at `p` means (texts have no edges: the store's hits, the most specific first).
fn pick(p: &Pointer, cx: &Context<'_>) -> Option<Slot> {
    let kinds = cx.draft.select_kinds;
    cx.spatial
        .hits(p.raw, cx.pick_tolerance())
        .into_iter()
        .find(|s| {
            cx.doc.get(*s).is_some_and(|e| {
                crate::selectable::entity_allowed(kinds, e) && placeable(e, cx.doc)
            })
        })
}

/// A dimension as the core lays it out.
fn dimension_geom(d: &kentos_contracts::DimensionEntity) -> DimensionGeom {
    let v = |p: &kentos_contracts::Vec2| Vec2::new(p.x, p.y);
    DimensionGeom {
        a: v(&d.a),
        b: v(&d.b),
        offset: d.offset,
        height: d.height,
        style: d.style.map(|s| s.name().to_owned()),
        angle: d.angle,
        c: d.c.as_ref().map(v),
        za: d.za,
        zb: d.zb,
        look: kentos_native_application::geometry::core_dimension_look(d),
    }
}

/// Ölçekteki yeri.
#[derive(Clone, Debug, Default)]
pub struct AnnotativePlace {
    /// The object picked, and the scale it goes at.
    target: Option<(Slot, Entity, f64)>,
    hover: Option<Vec2>,
    /// The representation at the scale with the cursor's place.
    shown: Option<Entity>,
}

impl AnnotativePlace {
    pub fn new() -> Self {
        Self::default()
    }

    /// The command's input that puts the target at `at`.
    fn request(&self, at: Vec2, cx: &Context<'_>) -> Option<AnnotationsScales> {
        let (slot, e, scale) = self.target.as_ref()?;
        let mut input = AnnotationsScales {
            operation: AnnotativeOperation::Set,
            uids: vec![cx.doc.uid(*slot)?.to_string()],
            scales: Vec::new(),
            scale: Some(*scale),
            point: None,
            rotation: None,
            offset: None,
            text_shift: None,
            expected_revision: None,
        };
        match e {
            Entity::Dimension(d) => {
                let shown = match at_scale(e, *scale) {
                    AtScale::Drawn(x) => *x,
                    _ => Entity::Dimension(d.clone()),
                };
                let Entity::Dimension(d) = shown else {
                    return None;
                };
                input.offset = Some(dimension_offset_at(&dimension_geom(&d), at));
            }
            _ => input.point = Some(wire(at)),
        }
        Some(input)
    }

    /// The representation the request would draw.
    fn preview_of(&self, input: &AnnotationsScales) -> Option<Entity> {
        let (_, e, scale) = self.target.as_ref()?;
        let mut entry = annotative_of(e)?.entry(*scale)?.clone();
        if input.point.is_some() {
            entry.point = input.point;
        }
        if input.offset.is_some() {
            entry.offset = input.offset;
        }
        let mut probe = e.clone();
        let field = kentos_contracts::annotative_field(&mut probe)?;
        let a = field.as_mut()?;
        for s in &mut a.scales {
            if same_scale(s.scale, *scale) {
                *s = entry.clone();
            }
        }
        match at_scale(&probe, *scale) {
            AtScale::Drawn(x) => Some(*x),
            _ => None,
        }
    }

    fn others(&self) -> Vec<f64> {
        self.target
            .as_ref()
            .and_then(|(_, e, _)| annotative_of(e))
            .map(|a| a.scales.iter().map(|s| s.scale).collect())
            .unwrap_or_default()
    }
}

impl Tool for AnnotativePlace {
    fn id(&self) -> &'static str {
        PLACE_ID
    }

    fn label(&self) -> &'static str {
        "Ölçekteki yeri"
    }

    fn prompt(&self) -> Prompt {
        match &self.target {
            None => Prompt::new(
                self.label(),
                "ölçekli yazıya, ölçüye ya da kılavuza tıklayın",
            ),
            Some((_, _, scale)) => Prompt::new(
                self.label(),
                "nesnenin bu ölçekteki yerini belirtin",
            )
            .option_with("Ölçek", "Ö", ratio(*scale)),
        }
    }

    fn point_count(&self) -> usize {
        usize::from(self.target.is_some())
    }

    fn cursor(&self) -> Cursor {
        if self.target.is_none() {
            Cursor::Pick
        } else {
            Cursor::Cross
        }
    }

    fn snaps(&self) -> bool {
        self.target.is_some()
    }

    fn accepts_points(&self) -> bool {
        self.target.is_some()
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        let Some(input) = self.request(p, cx) else {
            return false;
        };
        let words = said(annotations_scales::execute(
            &mut ExecutionContext::new(cx.doc),
            input,
        ));
        if words.is_empty() {
            let scale = self.target.as_ref().map_or(0.0, |t| t.2);
            cx.say(Level::Success, format!("{} yeri yazıldı", ratio(scale)));
        }
        for w in words {
            cx.say(Level::Warn, w);
        }
        self.target = None;
        self.shown = None;
        cx.selection.set_hover(None);
        true
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        if self.target.is_none() {
            let hit = pick(p, cx);
            cx.selection.set_hover(hit);
            self.shown = None;
            return;
        }
        self.shown = self
            .request(p.world, cx)
            .and_then(|input| self.preview_of(&input));
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.target.is_some() {
            self.accept_point(p.world, cx);
            return;
        }
        let Some(slot) = pick(p, cx) else {
            cx.say(
                Level::Warn,
                "Başka ölçeği olan ölçekli bir yazıya, ölçüye ya da kılavuza tıklayın (Öznitelikler › Ölçekler).",
            );
            return;
        };
        let Some(e) = cx.doc.get(slot).cloned() else {
            return;
        };
        let Some(first) = annotative_of(&e)
            .and_then(|a| a.scales.first())
            .map(|s| s.scale)
        else {
            return;
        };
        self.target = Some((slot, e, first));
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let text = js_trim(text);
        if self.target.is_some() && (text.eq_ignore_ascii_case("ö") || text == "Ö") {
            let others = self.others();
            if let Some((_, _, scale)) = self.target.as_mut() {
                let at = others
                    .iter()
                    .position(|s| same_scale(*s, *scale))
                    .unwrap_or(0);
                *scale = others[(at + 1) % others.len().max(1)];
            }
            return true;
        }
        if !self.accepts_points() {
            return false;
        }
        let Some(p) = cx.typed_point(text, None, self.hover) else {
            return false;
        };
        self.accept_point(p, cx)
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        if key != "Ö" {
            return Vec::new();
        }
        let now = self.target.as_ref().map_or(0.0, |t| t.2);
        self.others()
            .into_iter()
            .map(|s| OptionChoice {
                label: ratio(s),
                typed: ratio(s),
                icon: None,
                preview: None,
                checked: same_scale(s, now),
                command: None,
            })
            .collect()
    }

    fn choose_option(&mut self, key: &str, typed: &str, _cx: &mut Context<'_>) -> bool {
        if key != "Ö" {
            return false;
        }
        let Some(s) = self
            .others()
            .into_iter()
            .find(|s| ratio(*s) == js_trim(typed))
        else {
            return false;
        };
        if let Some(t) = self.target.as_mut() {
            t.2 = s;
        }
        true
    }

    fn takes_words(&self) -> bool {
        false
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.target.take().is_some() {
            self.shown = None;
            cx.selection.set_hover(None);
            return true;
        }
        false
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        let Some(shown) = &self.shown else {
            return Preview::default();
        };
        let scale = self.target.as_ref().map_or(0.0, |t| t.2);
        let mut strokes = Vec::new();
        let mut tag_at = self.hover;
        let mut words = vec![ratio(scale)];
        match shown {
            Entity::Dimension(d) => {
                if let Some(l) = layout_dimension(&dimension_geom(d)) {
                    strokes.extend(
                        l.lines
                            .iter()
                            .map(|&[p, q]| Stroke::solid(vec![p, q], false)),
                    );
                }
            }
            Entity::Leader(l) => {
                let pts: Vec<Vec2> = l.pts.iter().map(|p| Vec2::new(p.x, p.y)).collect();
                strokes.push(Stroke::solid(pts, false));
                if let Some(t) = &l.text {
                    words.insert(0, t.clone());
                }
                tag_at = l.pts.last().map(|p| Vec2::new(p.x, p.y));
            }
            Entity::Text(t) => {
                words.insert(0, t.text.lines().next().unwrap_or_default().to_owned());
                tag_at = Some(Vec2::new(t.p.x, t.p.y));
            }
            _ => {}
        }
        Preview {
            strokes,
            tag: tag_at.map(|at| Tag { at, lines: words }),
            ..Preview::default()
        }
    }
}
