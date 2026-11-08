//! Doğrama kodları (docs/adr/3028; ADV-16, ADV-03): the drawing's doors, windows and bare openings get the joinery
//! codes Mimarlar Odası's and İTÜ's notes give them, one for each type (K01 a door, GK01 a door in the outer wall,
//! P01 a window, B01 a bare opening), and their tags write the code over the size, 90/210.
//!
//! The codes are shown at the openings before anything is written, with the card's list of types; Enter (or a click)
//! writes them (`cad.openingtypes.edit`'s Codes) and, with Etiketle on, tags the openings that have none on the tags'
//! layer and rewrites those that have one (`cad.tags.create`, `{code}` over `{size}`): one undo step.

use kentos_commands::{
    Entity, NewTag, OpeningTypesEdit, OpeningTypesOperation, TagAll, TaggedKind, TagsCreate,
};
use kentos_native_application::opening_codes::{self, Plan};
use kentos_native_application::{ExecutionContext, opening_types_edit, tags};

use crate::Vec2;
use crate::arch_layer::Kind;
use crate::format::Format;
use crate::log::Level;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Label, Pointer, Preview, Tag, Tone, Tool};
use crate::tool_settings::Setting;

pub const ID: &str = "openingCodes";
pub const LABEL: &str = "Doğrama kodları";
/// The tag's words: the code over the size (docs/adr/3028).
pub const CONTENT: &str = "{code}\n{size}";

pub struct OpeningCodes {
    planned: Plan,
    seen: Option<u64>,
    /// Etiketle: the openings' tags written with the codes.
    tag: bool,
    at: Option<Vec2>,
}

impl Default for OpeningCodes {
    fn default() -> Self {
        Self::new()
    }
}

impl OpeningCodes {
    pub fn new() -> Self {
        Self {
            planned: Plan::default(),
            seen: None,
            tag: true,
            at: None,
        }
    }

    fn plan(&mut self, cx: &Context<'_>) {
        self.planned = opening_codes::plan(cx.doc);
        self.seen = Some(cx.doc.revision());
        // Before the cursor moves, the card stands over the coded openings.
        let (mut x0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY);
        for c in &self.planned.coded {
            x0 = x0.min(c.at.x);
            y1 = y1.max(c.at.y);
        }
        if self.at.is_none() {
            self.at = x0.is_finite().then(|| Vec2::new(x0, y1 + 1.0));
        }
    }

    /// The types coded, each with its code, name and how many of the drawing's openings are of it.
    fn types(&self) -> Vec<(String, String, usize)> {
        let mut out: Vec<(String, String, usize)> = Vec::new();
        for c in &self.planned.coded {
            match out.iter_mut().find(|(code, ..)| *code == c.code) {
                Some((.., n)) => *n += 1,
                None => out.push((c.code.clone(), c.type_name.clone(), 1)),
            }
        }
        out
    }

    fn lines(&self) -> Vec<String> {
        let types = self.types();
        let mut lines = Vec::new();
        if types.is_empty() {
            lines.push("Kod verilecek kapı, pencere ya da boşluk yok.".into());
            return lines;
        }
        let given = self.planned.given.len();
        lines.push(format!(
            "{} tip · {} açıklık · {} yeni kod",
            types.len(),
            self.planned.coded.len(),
            given
        ));
        for (code, name, n) in types.iter().take(8) {
            lines.push(format!("{code}  {name} · {n}"));
        }
        if types.len() > 8 {
            lines.push(format!("… {} tip daha", types.len() - 8));
        }
        if self.planned.locked > 0 {
            lines.push(format!(
                "{} açıklık kilitli katmanda: atlandı",
                self.planned.locked
            ));
        }
        lines.push(if self.tag {
            "Enter ya da tık: kodları yaz ve etiketle".into()
        } else {
            "Enter ya da tık: kodları yaz".into()
        });
        lines
    }

    /// Writes the codes and, with Etiketle, the tags: one undo step.
    fn write(&mut self, cx: &mut Context<'_>) -> bool {
        if self.seen != Some(cx.doc.revision()) {
            self.plan(cx);
        }
        if self.planned.coded.is_empty() {
            cx.say(Level::Warn, "Kod verilecek kapı, pencere ya da boşluk yok.");
            return false;
        }
        let group = cx.doc.begin_group(LABEL);
        let mut ok = true;
        if !self.planned.given.is_empty() || !self.planned.updates.is_empty() {
            let input = OpeningTypesEdit {
                operation: OpeningTypesOperation::Codes {},
                on_refused: Default::default(),
                expected_revision: None,
            };
            let result = opening_types_edit::execute(&mut ExecutionContext::new(cx.doc), input);
            ok = crate::points::written(result, cx).is_some();
        }
        let mut tagged = false;
        if ok && self.tag {
            ok = crate::arch_layer::writing(&Kind::Tag, cx, |id, cx| {
                // The untagged openings' tags placed clear of the walls, swings and tags (docs/adr/3028); the tagged
                // ones keep their places and take the new words.
                let doc = &*cx.doc;
                let have_tags: std::collections::BTreeSet<String> = doc
                    .entities()
                    .filter_map(|e| match e {
                        Entity::Tag(t) => Some(t.tag.of.clone()),
                        _ => None,
                    })
                    .collect();
                let untagged: Vec<String> = doc
                    .entities()
                    .filter(|e| {
                        e.kind() == "opening" && doc.layers().is_visible(&e.base().layer_id)
                    })
                    .filter_map(|e| {
                        doc.uid(kentos_domain::Slot(e.base().id))
                            .map(|u| u.to_string())
                    })
                    .filter(|u| !have_tags.contains(u))
                    .collect();
                let placed = tags::opening_places(doc, &untagged, Some(CONTENT), None, None);
                let input = TagsCreate {
                    layer_id: id.to_owned(),
                    tags: placed
                        .into_iter()
                        .map(|(of, at)| NewTag {
                            of,
                            at: Some(at),
                            content: Some(CONTENT.into()),
                            shape: None,
                            size: None,
                            leader: None,
                        })
                        .collect(),
                    all: Some(TagAll {
                        kinds: vec![TaggedKind::Opening],
                        refresh: Some(true),
                        layers: None,
                        levels: None,
                        content: Some(CONTENT.into()),
                        shape: None,
                        size: None,
                        phases: None,
                    }),
                    color: None,
                    line_weight: None,
                    expected_revision: None,
                };
                let result = tags::create::execute(&mut ExecutionContext::new(cx.doc), input);
                tagged = crate::points::written(result, cx).is_some();
                tagged
            });
        }
        if !ok {
            cx.doc.cancel_group(group);
            self.seen = None;
            return false;
        }
        cx.doc.end_group(group);
        let types = self.types();
        let list = types
            .iter()
            .map(|(code, name, _)| format!("{code} {name}"))
            .collect::<Vec<_>>()
            .join(", ");
        cx.say(
            Level::Success,
            format!(
                "Doğrama kodları verildi: {list}{}; tek geri almayla gider.",
                if tagged {
                    "; açıklıklar etiketlendi"
                } else {
                    ""
                }
            ),
        );
        self.seen = None;
        true
    }
}

impl Tool for OpeningCodes {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        Prompt::new(LABEL, "önizlemeye bakın; Enter ya da tık ile yazın").option_with(
            "Etiketle",
            "E",
            if self.tag { "Açık" } else { "Kapalı" },
        )
    }

    /// It takes no points: the codes are the drawing's.
    fn point_count(&self) -> usize {
        0
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.plan(cx);
        Flow::Stay
    }

    fn layer_kind(&self, _cx: &Context<'_>) -> Option<Kind> {
        self.tag.then_some(Kind::Tag)
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.seen != Some(cx.doc.revision()) {
            self.plan(cx);
        }
        // The card follows the cursor (docs/adr/3018): moved aside, it leaves the plan in sight.
        self.at = Some(p.raw);
    }

    fn pointer_down(&mut self, _p: &Pointer, cx: &mut Context<'_>) {
        self.write(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if upper_tr(text.trim()) == "E" {
            self.tag = !self.tag;
            self.plan(cx);
            return true;
        }
        false
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.write(cx) {
            Flow::Exit
        } else {
            Flow::Stay
        }
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = Preview::default();
        for c in &self.planned.coded {
            out.labels.push(Label {
                at: Vec2::new(c.at.x, c.at.y),
                text: c.code.clone(),
                offset: [6.0, -6.0],
                tone: Tone::Accent,
            });
            out.marks.push(Vec2::new(c.at.x, c.at.y));
        }
        out.tag = self.at.map(|at| Tag {
            at,
            lines: self.lines(),
        });
        out
    }

    fn settings(&self, _format: &Format) -> Vec<Setting> {
        vec![Setting::toggle("E", "Etiketle (kod ve 90/210)", self.tag)]
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        if key != "E" {
            return false;
        }
        self.tag = matches!(
            crate::prompt::fold_tr(value.trim()).as_str(),
            "YES" | "AÇIK" | "ACIK" | "EVET" | "1" | "TRUE"
        );
        self.plan(cx);
        true
    }
}
