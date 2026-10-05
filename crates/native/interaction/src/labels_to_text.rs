//! Etiketleri yazıya çevir (docs/adr/0175 §3; Netcad's Etiketleri CAD'e
//! çevir, ArcGIS' Convert Labels To Annotation, QGIS' Extract labels): the
//! layers' labels written as text objects, as a sheet at 1:N writes them.
//! The rule is the shared core's (`ops::label_text`, through the store's
//! `label_texts`); the web's tool is `apps/web/src/tools/labelsToTextTool.ts`,
//! and both play `fixtures/interaction/v1/labels-to-text.json`.
//!
//! - **Scope**, taken when it starts: the selection's labels, else every
//!   labelled object's on a visible layer; a label's style is its layer's,
//!   else its kind's default, as the drawing shows it. Texts, dimensions and
//!   leaders show none, and an object whose label a text writes already
//!   (docs/adr/0175 §4) has its text for a label.
//! - **Options**, as Topolojik temizlik's: a number typed is the scale (Ö
//!   asks for it; each run starts at the project's drawing scale);
//!   Örtüşenler de (R), Zemin (Z), Katman (K: the standard text layer or
//!   the active one) and Nesneye bağlı (B: the texts know their objects and
//!   follow them, `labelOf`, `labelScale`) are kept for as long as the app
//!   lives ([`crate::tool::Memory`]).
//! - **Shown first**: the texts in place, faint, and the counts beside the
//!   cursor. Enter, the Uygula button or a quick right click writes them
//!   through `cad.entities.create` (`labels`) in one step, opening the text
//!   layer in that step when the drawing lacks it, selects them and leaves;
//!   Esc leaves.

use kentos_contracts::{
    CreateOperation, EntitiesCreate, Entity, EntityGeometry, LabelPlacement, LabelStyle, NewObject,
    TextAlign,
};
use kentos_domain::Slot;
use kentos_geometry_core::ops::label_text::LabelTexts;
use kentos_geometry_core::store::labels::{LabelLook, LabelWanted, Placement};
use kentos_native_application::{ExecutionContext, create};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::spatial::default_label;
use crate::standard_layer;
use crate::tool::{Context, Cursor, Flow, Preview, Tag, TextGhost, Tool};

/// Etiketleri yazıya çevir's id: its command is `tool.labelsToText`.
pub const ID: &str = "labelsToText";
pub const LABEL: &str = "Etiketleri yazıya çevir";
/// The standard text layer the texts go to unless the active layer is chosen (docs/adr/0175 §3).
pub const TEXT_LAYER: &str = "yazi";
/// The texts shown at most.
const MAX_SHOWN: usize = 2000;

/// The tool.
#[derive(Clone, Debug, Default)]
pub struct LabelsToText {
    /// The labels it converts, taken when it starts, in the drawing's order.
    wanted: Vec<LabelWanted>,
    whole: bool,
    /// The scale's denominator: the project's when the tool starts.
    scale: u64,
    /// Ö: the scale is being typed.
    typing: bool,
    /// What was worked out, and from what: the drawing's generation, the scale, the thinning.
    plan: Option<(u64, u64, bool, LabelTexts)>,
    /// The options and the text layer's name as of the last event, for the prompt and the preview.
    seen: Option<Seen>,
    /// The finding last said, so a change that finds the same says nothing again.
    said: String,
    /// The cursor's world point: the tag beside it says the finding.
    hover: Option<Vec2>,
    done: bool,
}

/// The options and the text layer's name as of the last event.
#[derive(Clone, Debug, Default)]
struct Seen {
    every: bool,
    mask: bool,
    linked: bool,
    layer: String,
}

impl LabelsToText {
    pub fn new() -> Self {
        Self::default()
    }

    /// The labels to convert (docs/adr/0175 §3); false when there are none, said.
    fn take_scope(&mut self, cx: &mut Context<'_>) -> bool {
        let doc = &*cx.doc;
        let layers = doc.layers();
        self.whole = cx.selection.is_empty();
        let chosen: Vec<&Entity> = if self.whole {
            doc.entities().collect()
        } else {
            cx.selection
                .ids()
                .iter()
                .filter_map(|slot| doc.get(*slot))
                .collect()
        };
        self.wanted = chosen
            .into_iter()
            .filter_map(|e| {
                let base = e.base();
                let label = base.label.as_deref().filter(|l| !l.is_empty())?;
                // A text writes its label already (docs/adr/0175 §4).
                if matches!(
                    e,
                    Entity::Text(_) | Entity::Dimension(_) | Entity::Leader(_)
                ) || !layers.is_visible(&base.layer_id)
                    || doc.has_linked_text(Slot(base.id))
                {
                    return None;
                }
                let style = layers
                    .get(&base.layer_id)
                    .and_then(|l| l.style.label.clone())
                    .or_else(|| default_label(e.kind()))?;
                Some(LabelWanted {
                    id: f64::from(base.id),
                    label: label.to_owned(),
                    style: look(&style),
                })
            })
            .collect();
        if self.wanted.is_empty() {
            let place = if self.whole { "çizimde" } else { "seçimde" };
            cx.say(
                Level::Warn,
                format!("{LABEL}: {place} yazıya çevrilecek etiket yok."),
            );
        }
        !self.wanted.is_empty()
    }

    /// The texts for the drawing as it is, kept until the drawing, the scale
    /// or the thinning change; the options for the prompt. Whether it was
    /// worked out again.
    fn refresh(&mut self, cx: &Context<'_>) -> bool {
        let m = &cx.memory;
        let layer = if m.labels_active {
            let active = cx.doc.layers().active();
            cx.doc
                .layers()
                .get(active)
                .map_or_else(String::new, |l| l.name.clone())
        } else {
            standard_layer::name_of(TEXT_LAYER, cx)
        };
        self.seen = Some(Seen {
            every: m.labels_every,
            mask: m.labels_mask,
            linked: m.labels_linked,
            layer,
        });
        let generation = cx.doc.generation();
        let thin = !m.labels_every;
        if self
            .plan
            .as_ref()
            .is_some_and(|(g, s, t, _)| *g == generation && *s == self.scale && *t == thin)
        {
            return false;
        }
        let texts = cx
            .spatial
            .store()
            .label_texts(&self.wanted, self.scale as f64, thin);
        self.plan = Some((generation, self.scale, thin, texts));
        true
    }

    fn texts(&self) -> Option<&LabelTexts> {
        self.plan.as_ref().map(|(_, _, _, t)| t)
    }

    /// Says the finding when it changed; when the tool starts, with its
    /// scope and what to do (the web's `tell`).
    fn tell(&mut self, cx: &mut Context<'_>, scope: Option<String>) {
        let Some(r) = self.texts() else { return };
        let text = finding(r, self.scale);
        if scope.is_none() && text == self.said {
            return;
        }
        let found = !r.texts.is_empty();
        let head = match &scope {
            Some(scope) => format!("{LABEL}: {scope}; {text}"),
            None => format!("{LABEL}: {text}"),
        };
        let message = match (found, scope.is_some()) {
            (true, true) => format!("{head}. Enter ile yazın."),
            (true, false) => format!("{head}."),
            (false, _) => format!("{head}; başka bir ölçek yazın."),
        };
        cx.say(if found { Level::Info } else { Level::Warn }, message);
        self.said = text;
    }

    /// Writes the texts in one step and says what came of it.
    fn write(&mut self, cx: &mut Context<'_>) -> Flow {
        self.refresh(cx);
        let Some(r) = self.texts().cloned() else {
            return Flow::Exit;
        };
        if r.texts.is_empty() {
            cx.say(
                Level::Warn,
                format!("{LABEL}: yazılacak etiket yok; hiçbir şey değişmedi."),
            );
            return Flow::Exit;
        }
        let mask = cx.memory.labels_mask;
        // Nesneye bağlı: each text knows its object and the scale (docs/adr/0175 §4).
        let scale = self.scale as f64;
        let link = |item: usize| {
            let slot = Slot(self.wanted.get(item)?.id as u32);
            let uid = cx.doc.uid(slot)?.to_string();
            Some((uid, scale))
        };
        let linked = cx.memory.labels_linked;
        let objects = r
            .texts
            .iter()
            .map(|t| {
                let (label_of, label_scale) = match linked.then(|| link(t.item)).flatten() {
                    Some((uid, scale)) => (Some(uid), Some(scale)),
                    None => (None, None),
                };
                NewObject {
                    geometry: EntityGeometry::Text {
                        p: kentos_contracts::Vec2 { x: t.p.x, y: t.p.y },
                        text: t.text.clone(),
                        height: t.height,
                        rotation: t.rotation,
                        align: contract_align(t.align),
                        width_factor: None,
                        mask,
                    },
                    color: None,
                    line_weight: None,
                    attrs: None,
                    label: None,
                    label_of,
                    label_scale,
                }
            })
            .collect();
        let active = cx.memory.labels_active;
        let layer_id = if active {
            cx.doc.layers().active().to_owned()
        } else {
            TEXT_LAYER.to_owned()
        };
        let input = EntitiesCreate {
            layer_id,
            objects,
            operation: Some(CreateOperation::Labels),
            expected_revision: None,
        };
        // A drawing without the text layer gets it, in the texts' own undo step.
        let opened = if active {
            None
        } else {
            match standard_layer::open_in_step(TEXT_LAYER, "etiketlerin yazıları", LABEL, cx) {
                Ok(opened) => opened,
                Err(()) => return Flow::Stay,
            }
        };
        let result = create::execute(&mut ExecutionContext::new(cx.doc), input);
        let out = points::written(result, cx);
        match (&out, opened) {
            (Some(_), Some(opened)) => opened.keep(cx),
            (None, Some(opened)) => opened.drop(cx),
            (_, None) => {}
        }
        let Some(out) = out else {
            return Flow::Stay;
        };
        cx.selection
            .set(out.ids.iter().map(|&id| Slot(id)).collect::<Vec<_>>());
        cx.say(
            Level::Success,
            format!(
                "{LABEL}: {} etiket yazıya çevrildi{}.",
                r.texts.len(),
                skipped(&r, true)
            ),
        );
        self.done = true;
        Flow::Exit
    }
}

/// The label style's parts the store reads (docs/adr/0175 §1).
fn look(s: &LabelStyle) -> LabelLook {
    LabelLook {
        placement: match s.placement {
            LabelPlacement::Center => Placement::Center,
            LabelPlacement::Corner => Placement::Corner,
            LabelPlacement::Beside => Placement::Beside,
            LabelPlacement::Along => Placement::Along,
        },
        size: s.size,
        grow: s.grow,
        max_size: s.max_size,
        template: s.template.clone(),
        min_feature_px: s.min_feature_px,
        min_scale: s.min_scale,
        max_scale: s.max_scale,
    }
}

/// The core's alignment as the contract names it (the same names).
fn contract_align(a: kentos_geometry_core::text::TextAlign) -> Option<TextAlign> {
    TextAlign::ALL.into_iter().find(|c| c.name() == a.name())
}

/// `1:500`, `500`: a scale's denominator, a whole number from 1; none for anything else.
pub fn read_scale(text: &str) -> Option<u64> {
    let t = text.trim();
    let t = match t.strip_prefix('1') {
        Some(rest) if rest.trim_start().starts_with(':') => rest.trim_start()[1..].trim(),
        _ => t,
    };
    if t.is_empty() || !t.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let n: u64 = t.parse().ok()?;
    // As the web's: a whole number JavaScript holds exactly.
    (1..=9_007_199_254_740_991).contains(&n).then_some(n)
}

/// `, 1 örtüşen, 2 ölçek dışı etiket atlanacak` (`past`: atlandı); empty when none is.
fn skipped(r: &LabelTexts, past: bool) -> String {
    let parts: Vec<String> = [
        (r.overlapping, "örtüşen"),
        (r.out_of_scale, "ölçek dışı"),
        (r.small, "küçük"),
    ]
    .iter()
    .filter(|(n, _)| *n > 0)
    .map(|(n, what)| format!("{n} {what}"))
    .collect();
    if parts.is_empty() {
        return String::new();
    }
    let verb = if past { "atlandı" } else { "atlanacak" };
    format!(", {} etiket {verb}", parts.join(", "))
}

/// `1:1000 ölçekte 4 yazı olacak, 1 örtüşen etiket atlanacak`: no suffix on
/// the number, whose sound decides it.
fn finding(r: &LabelTexts, scale: u64) -> String {
    let count = if r.texts.is_empty() {
        "yazı olacak etiket yok".to_owned()
    } else {
        format!("{} yazı olacak", r.texts.len())
    };
    format!("1:{scale} ölçekte {count}{}", skipped(r, false))
}

/// The finding beside the cursor, a count a line, and what writes it.
fn tag_lines(r: &LabelTexts, scale: u64) -> Vec<String> {
    let mut lines = vec![format!("{} yazı, 1:{scale}", r.texts.len())];
    for (n, what) in [
        (r.overlapping, "örtüşen"),
        (r.out_of_scale, "ölçek dışı"),
        (r.small, "küçük"),
    ] {
        if n > 0 {
            lines.push(format!("{n} {what} atlanır"));
        }
    }
    lines.push(if r.texts.is_empty() {
        "Yazılacak etiket yok".to_owned()
    } else {
        "Enter: yaz".to_owned()
    });
    lines
}

impl Tool for LabelsToText {
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

    fn prompt(&self) -> Prompt {
        if self.typing {
            return Prompt::new(
                LABEL,
                format!("ölçeği 1:N ya da N olarak yazın (Enter: 1:{})", self.scale),
            );
        }
        let Seen {
            every,
            mask,
            linked,
            layer,
        } = self.seen.clone().unwrap_or_default();
        let on = |b: bool| if b { "açık" } else { "kapalı" };
        let step = self
            .texts()
            .map_or_else(String::new, |r| finding(r, self.scale));
        Prompt::new(LABEL, step)
            .option_with("Ölçek", "Ö", format!("1:{}", self.scale))
            .option_with("Örtüşenler de", "R", on(every))
            .option_with("Zemin", "Z", on(mask))
            .option_with("Katman", "K", layer)
            .option_with("Nesneye bağlı", "B", on(linked))
            .option("Uygula", "Enter")
    }

    fn point_count(&self) -> usize {
        0
    }

    /// Takes the scope and says what it found; with no label, says so and leaves.
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.scale = cx.doc.settings().plot_scale.round().max(1.0) as u64;
        if !self.take_scope(cx) {
            return Flow::Exit;
        }
        self.refresh(cx);
        let place = if self.whole {
            "bütün çizimde"
        } else {
            "seçimde"
        };
        self.tell(cx, Some(format!("{place} {} etiket", self.wanted.len())));
        Flow::Stay
    }

    /// The tag follows the cursor; a drawing changed under the tool (an undo) is worked out again.
    fn pointer_move(&mut self, p: &crate::Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.raw);
        if self.refresh(cx) {
            self.tell(cx, None);
        }
    }

    /// A click does nothing: Enter, the button or a quick right click write.
    fn pointer_down(&mut self, _p: &crate::Pointer, _cx: &mut Context<'_>) {}

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = upper_tr(crate::js_trim(text));
        if !self.typing {
            let flag = match key.as_str() {
                "R" => Some(&mut cx.memory.labels_every),
                "Z" => Some(&mut cx.memory.labels_mask),
                "K" => Some(&mut cx.memory.labels_active),
                "B" => Some(&mut cx.memory.labels_linked),
                _ => None,
            };
            if let Some(flag) = flag {
                *flag = !*flag;
                self.refresh(cx);
                self.tell(cx, None);
                return true;
            }
            if key == "Ö" || key == "O" {
                self.typing = true;
                self.refresh(cx);
                return true;
            }
        }
        let Some(n) = read_scale(text) else {
            if !self.typing {
                return false;
            }
            cx.say(
                Level::Warn,
                "Ölçeği 1:N ya da N olarak, 1 ya da daha büyük bir tam sayıyla yazın (1:500, 1000)."
                    .to_owned(),
            );
            return true;
        };
        self.scale = n;
        self.typing = false;
        self.refresh(cx);
        self.tell(cx, None);
        true
    }

    /// Enter: the scale kept while it is typed; otherwise the texts written, and the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.typing {
            self.typing = false;
            self.refresh(cx);
            return Flow::Stay;
        }
        self.write(cx)
    }

    /// Esc while the scale is typed goes back to the finding; otherwise the tool leaves.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if !self.typing {
            return false;
        }
        self.typing = false;
        self.refresh(cx);
        true
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn finished(&self) -> bool {
        self.done
    }

    /// The texts in place, faint, over their masks when they will have
    /// them; the counts beside the cursor.
    fn preview(&self, _format: &Format) -> Preview {
        let Some(r) = self.texts() else {
            return Preview::default();
        };
        let mask = self.seen.as_ref().is_some_and(|s| s.mask);
        Preview {
            texts: r
                .texts
                .iter()
                .take(MAX_SHOWN)
                .map(|t| TextGhost {
                    p: t.p,
                    text: t.text.clone(),
                    height: t.height,
                    rotation: t.rotation,
                    align: t.align,
                    mask,
                })
                .collect(),
            tag: self.hover.map(|at| Tag {
                at,
                lines: tag_lines(r, self.scale),
            }),
            ..Preview::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scale_reads_as_one_to_n_or_n() {
        let got: Vec<Option<u64>> = [
            "1:500",
            " 1 : 2500 ",
            "1000",
            "1:0",
            "0",
            "1:5.5",
            "2:500",
            "",
        ]
        .iter()
        .map(|t| read_scale(t))
        .collect();
        assert_eq!(
            got,
            [
                Some(500),
                Some(2500),
                Some(1000),
                None,
                None,
                None,
                None,
                None
            ]
        );
    }
}
