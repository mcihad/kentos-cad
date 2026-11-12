//! Revizyon bulutu: the web's `RevCloudTool` (AutoCAD REVCLOUD,
//! `apps/web/src/tools/markupTools.ts`) on its `PointInputTool` base, step
//! for step (docs/adr/0057):
//!
//! - a rectangle's two corners (the default), or with Çokgen (Ç, or C) a
//!   polygon's corners ended by Enter or a quick right click; Dikdörtgen (D)
//!   goes back; both before the first corner; Nesne (N, AutoCAD's Object,
//!   docs/adr/0287): every closed object clicked (an area, a closed polyline,
//!   a circle, a closed curve, a full ellipse) gets a cloud round its outer
//!   outline (`cloud_around`), the object stays;
//! - Yay boyu (U): the arcs' length in paper millimetres at the project's
//!   plot scale; the shape and the length stay for as long as the app lives.
//!
//! The cloud is a closed area of outward arcs written by `cad.revcloud.create`
//! (docs/adr/0622 §9), its own object and undo step (“Ekle”): “Revizyon
//! bulutu eklendi: 24 yay.” The tool gives the rectangle's corners, the
//! polygon's or the object's persistent id and the arc in metres (paper
//! millimetres at the plot scale); the preview is the command's ring
//! (`shapes::revcloud_ring`). With Çakışma or Topoloji changing the area it is
//! written as any area is (`points::write_area_by`).
//!
//! Revizyon işaretle (docs/adr/0721 §4) is the same tool with a revision's number: Revizyon (R) types it (the
//! drawing's last delta's by default, else 1), Tarih (T) and Açıklama (A) its date and description (a number the
//! drawing has brings its revision's); the cloud and its delta are written by `cad.revisions.mark` in one undo step
//! (“Revizyon”), the delta's ghost beside the cloud's from the command's plan. Çakışma and Topoloji do not clip a
//! revision's cloud.

use kentos_commands::{CommandResult, Entity, RevcloudCreate, RevcloudFrame, RevisionsMark};
use kentos_geometry_core::geom::arc::DEFAULT_STEP;
use kentos_geometry_core::geom::bulge::bulge_path_outline;
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::{ExecutionContext, revisions, shapes};

use crate::Vec2;
use crate::format::{Format, js_number};
use crate::log::Level;
use crate::points::{self, Taken};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Memory, Pointer, Preview, Stroke, Tool};

/// The revision cloud tool's id: its command is `tool.revcloud`.
pub const ID: &str = "revcloud";
pub const LABEL: &str = "Revizyon bulutu";
/// Revizyon işaretle's id: its command is `tool.revisionMark` (docs/adr/0721 §4).
pub const MARK_ID: &str = "revisionMark";
pub const MARK_LABEL: &str = "Revizyon işaretle";

/// A description shortened for the prompt's chip: its first 24 characters and “…”.
fn short(text: &str) -> String {
    if text.chars().count() <= 24 {
        return text.to_owned();
    }
    let mut out: String = text.chars().take(23).collect();
    out.push('…');
    out
}

/// The revision cloud tool.
#[derive(Clone, Debug, Default)]
pub struct RevCloud {
    d: Taken,
    ask_arc: bool,
    /// Nesne: clicks pick closed objects.
    object: bool,
    /// What the session remembered and the project's plot scale, as of the last call.
    seen: Option<(Memory, f64)>,
    /// Revizyon işaretle: the revision's number (none until the tool has seen the drawing's last), its date and
    /// description, and the one of Revizyon (R), Tarih (T) and Açıklama (A) asked.
    mark: Option<Option<String>>,
    date: Option<String>,
    description: Option<String>,
    asking: Option<&'static str>,
    /// The delta's ghost at the cursor, from the command's plan.
    tag: Vec<Stroke>,
}

impl RevCloud {
    pub fn new() -> Self {
        Self::default()
    }

    /// Revizyon işaretle: the cloud and its delta with a revision's number.
    pub fn mark() -> Self {
        Self {
            mark: Some(None),
            ..Self::default()
        }
    }

    fn label_of(&self) -> &'static str {
        if self.mark.is_some() { MARK_LABEL } else { LABEL }
    }

    /// The revision's number a mark writes.
    fn number(&self) -> String {
        self.mark.clone().flatten().unwrap_or_else(|| "1".to_owned())
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.doc.settings().plot_scale));
        // The drawing's last revision, with its date and description, until one is typed.
        if self.mark == Some(None) {
            let number = revisions::last_number(cx.doc).unwrap_or_else(|| "1".to_owned());
            (self.date, self.description) = revisions::data_of(cx.doc, &number);
            self.mark = Some(Some(number));
        }
    }

    /// The mark's input for `frame`.
    fn mark_request(&self, frame: RevcloudFrame, cx: &Context<'_>) -> RevisionsMark {
        RevisionsMark {
            layer_id: cx.doc.layers().active().to_owned(),
            frame,
            arc: self.arc_length(),
            number: self.number(),
            date: self.date.clone(),
            description: self.description.clone(),
            at: None,
            size: None,
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            expected_revision: None,
        }
    }

    /// The delta's ghost for the corners so far and the cursor.
    fn see_tag(&mut self, cx: &Context<'_>) {
        self.tag.clear();
        if self.mark.is_none() {
            return;
        }
        let pts: Vec<Vec2> = self.d.pts.iter().copied().chain(self.d.hover).collect();
        if pts.len() < 2 {
            return;
        }
        let input = self.mark_request(self.frame_of(&pts), cx);
        if let CommandResult::Completed { output, .. } = revisions::plan_in(cx.doc, Some(cx.spatial.store()), &input) {
            let Entity::Insert(i) = &output.tag else { return };
            if let Some(def) = output.block.or_else(|| cx.doc.block(i.block).cloned()) {
                self.tag = crate::symbols::definition_ghost(&def, &output.tag);
            }
        }
    }

    fn memory(&self) -> Memory {
        self.seen.map_or_else(Memory::default, |(m, _)| m)
    }

    /// The arcs' length in world metres: paper millimetres at the plot scale.
    fn arc_length(&self) -> f64 {
        let (m, scale) = self.seen.unwrap_or((Memory::default(), 1000.0));
        (m.cloud_arc_mm / 1000.0) * scale
    }

    /// Dikdörtgen (D), Çokgen (Ç), Yay boyu (U) and, marking, Revizyon (R), Tarih (T) and Açıklama (A), before the
    /// first corner.
    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
        if !self.d.pts.is_empty() {
            return false;
        }
        match key {
            "R" if self.mark.is_some() => self.asking = Some("R"),
            "T" if self.mark.is_some() => self.asking = Some("T"),
            "A" if self.mark.is_some() => self.asking = Some("A"),
            "D" => {
                cx.memory.cloud_rect = true;
                self.object = false;
            }
            "Ç" | "C" => {
                cx.memory.cloud_rect = false;
                self.object = false;
            }
            "N" => self.object = true,
            "U" => self.ask_arc = true,
            _ => return false,
        }
        true
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.d.begin(p, cx);
        if self.ask_arc || self.d.last().is_some_and(|last| dist(last, p) <= 1e-9) {
            return;
        }
        self.d.pts.push(p);
        if cx.memory.cloud_rect && self.d.pts.len() == 2 {
            self.finish(cx);
        }
    }

    /// The command's input for `frame`, with the drawing's look and the template's attributes.
    fn request(&self, frame: RevcloudFrame, cx: &Context<'_>) -> RevcloudCreate {
        RevcloudCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            frame,
            arc: self.arc_length(),
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            line_type: cx.draft.line_type,
            attrs: cx.template_attrs(None),
            label: cx.template_label(),
            symbol: cx.template_symbol(),
            expected_revision: None,
            rules: None,
        }
    }

    /// The frame of the corners so far: the rectangle on two corners, or the corners.
    fn frame_of(&self, pts: &[Vec2]) -> RevcloudFrame {
        match (self.memory().cloud_rect, pts) {
            (true, [a, b, ..]) => RevcloudFrame::Rectangle {
                a: points::wire(*a),
                b: points::wire(*b),
            },
            _ => RevcloudFrame::Polygon {
                pts: pts.iter().copied().map(points::wire).collect(),
            },
        }
    }

    /// The cloud and its delta through `cad.revisions.mark` (docs/adr/0721 §4); a refusal says the command's reason.
    fn write_mark(&mut self, frame: RevcloudFrame, cx: &mut Context<'_>) {
        let input = self.mark_request(frame, cx);
        let number = input.number.clone();
        let result = revisions::execute(&mut ExecutionContext::with_store(cx.doc, cx.spatial.store()), input);
        if points::written(result, cx).is_some() {
            cx.say(Level::Success, format!("Revizyon {number} işaretlendi: bulut ve üçgen."));
        }
        self.tag.clear();
    }

    /// The cloud as the command makes it, written by the command unless Çakışma or Topoloji changes it; a refusal
    /// says the command's reason.
    fn write(&mut self, input: RevcloudCreate, cx: &mut Context<'_>) {
        if self.mark.is_some() {
            self.write_mark(input.frame, cx);
            return;
        }
        match shapes::revcloud_ring(Some(&*cx.doc), &input) {
            Ok((pts, _)) => {
                let arcs = pts.len();
                let mut input = input;
                let rules = points::area_rules(cx, &input.layer_id);
                input.rules = rules;
                let mut write = |cx: &mut Context<'_>| {
                    let result = shapes::execute_revcloud(
                        &mut ExecutionContext::with_store(cx.doc, cx.spatial.store()),
                        input.clone(),
                    );
                    points::written(result, cx).map(|out| out.id)
                };
                if points::write_area_by(&mut self.d, cx, &mut write).is_some() {
                    cx.say(
                        Level::Success,
                        format!("Revizyon bulutu eklendi: {arcs} yay."),
                    );
                }
            }
            Err(error) => cx.say(Level::Warn, error.message),
        }
    }

    /// Nesne: the cloud round the closed object under the press, by its persistent id.
    fn around(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        let uid = cx
            .spatial
            .pick_edge(p.raw, cx.pick_tolerance(), |s| cx.doc.get(s).is_some())
            .and_then(|slot| cx.doc.uid(slot))
            .map(|uid| uid.to_string());
        match uid {
            Some(uid) => {
                let input = self.request(RevcloudFrame::Object { uid }, cx);
                self.write(input, cx);
                self.d.reset();
            }
            None => cx.say(
                Level::Warn,
                "Kapalı bir nesneye tıklayın: alan, kapalı çoklu çizgi, daire, kapalı eğri ya da elips.",
            ),
        }
    }

    /// Writes the cloud through `cad.revcloud.create` (docs/adr/0622 §9); a
    /// refused one says only the command's reason. The draft starts over.
    fn finish(&mut self, cx: &mut Context<'_>) {
        self.see(cx);
        if !self.d.pts.is_empty() {
            let input = self.request(self.frame_of(&self.d.pts), cx);
            self.write(input, cx);
        }
        self.d.reset();
    }
}

impl Tool for RevCloud {
    /// A point computed by the point calculator, as if clicked (the web's `acceptPoint`).
    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.accept(p, cx);
        true
    }
    fn id(&self) -> &'static str {
        if self.mark.is_some() { MARK_ID } else { ID }
    }

    fn label(&self) -> &'static str {
        self.label_of()
    }

    fn prompt(&self) -> Prompt {
        let label = self.label_of();
        match self.asking {
            Some("R") => return Prompt::new(label, "revizyonun numarasını ya da harfini yazın (1, A, B2 gibi)"),
            Some("T") => return Prompt::new(label, "revizyonun tarihini yazın (2026-10-27 gibi; boş: yok)"),
            Some(_) => return Prompt::new(label, "revizyonda neyin değiştiğini yazın (boş: yok)"),
            None => {}
        }
        if self.ask_arc {
            return Prompt::new(label, "yay boyunu kâğıt milimetresi olarak yazın");
        }
        let m = self.memory();
        let n = self.d.pts.len();
        let object = self.object;
        let number = self.mark.as_ref().map(|_| self.number());
        let first = |step: &'static str| {
            let prompt = Prompt::new(label, step);
            let prompt = match &number {
                Some(n) => {
                    let prompt = prompt.option_with("Revizyon", "R", n.clone());
                    let prompt = match &self.date {
                        Some(d) => prompt.option_with("Tarih", "T", d.clone()),
                        None => prompt.option("Tarih", "T"),
                    };
                    match &self.description {
                        Some(d) => prompt.option_with("Açıklama", "A", short(d)),
                        None => prompt.option("Açıklama", "A"),
                    }
                }
                None => prompt,
            };
            let prompt = if object {
                prompt.option("Dikdörtgen", "D").option("Çokgen", "Ç")
            } else if m.cloud_rect {
                prompt.option("Çokgen", "Ç").option("Nesne", "N")
            } else {
                prompt.option("Dikdörtgen", "D").option("Nesne", "N")
            };
            prompt.option_with("Yay boyu", "U", format!("{} mm", js_number(m.cloud_arc_mm)))
        };
        if object {
            return first("buluta çevrilecek kapalı nesneye tıklayın");
        }
        match (m.cloud_rect, n) {
            (true, 0) => first("dikdörtgenin bir köşesine tıklayın"),
            (true, _) => Prompt::new(label, "karşı köşeye tıklayın"),
            (false, 0) => first("bulutun ilk köşesine tıklayın"),
            (false, 1 | 2) => Prompt::new(label, "sonraki köşeye tıklayın"),
            (false, _) => Prompt::new(
                label,
                "sonraki köşeye tıklayın ya da bitirmek için sağ tıklayın",
            ),
        }
    }

    fn point_count(&self) -> usize {
        self.d.pts.len()
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.d.last()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.d.hover = Some(self.d.constrain(p, cx));
        self.see(cx);
        self.see_tag(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.object && !self.ask_arc {
            self.around(p, cx);
            return;
        }
        let p = self.d.constrain(p, cx);
        self.accept(p, cx);
        self.see(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if let Some(key) = self.asking.take() {
            return self.configure(key, text, cx);
        }
        let done = if self.option(&upper_tr(js_trim(text)), cx) {
            true
        } else if let (true, Some(n)) = (self.ask_arc, parse_number(text)) {
            if n > 0.0 {
                cx.memory.cloud_arc_mm = n;
            } else {
                cx.say(Level::Warn, "Yay boyu sıfırdan büyük olmalı.");
            }
            self.ask_arc = false;
            true
        } else {
            match cx.typed_point(text, self.d.last(), self.d.hover) {
                Some(p) => {
                    self.accept(p, cx);
                    true
                }
                None => false,
            }
        };
        self.see(cx);
        done
    }

    /// Enter: the corners so far make the cloud; with none, the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.d.pts.is_empty() {
            return Flow::Exit;
        }
        self.finish(cx);
        Flow::Stay
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.d.undo_step(false, cx)
    }

    /// Revizyon işaretle's Revizyon, Tarih and Açıklama: what its next mark writes.
    fn settings(&self, _f: &Format) -> Vec<crate::tool_settings::Setting> {
        use crate::tool_settings::Setting;
        match &self.mark {
            Some(_) => vec![
                Setting::text("R", "Revizyon", self.number()),
                Setting::text("T", "Tarih", self.date.clone().unwrap_or_default()),
                Setting::text("A", "Açıklama", self.description.clone().unwrap_or_default()),
            ],
            None => Vec::new(),
        }
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        if self.mark.is_none() {
            return false;
        }
        let text = js_trim(value);
        match key {
            "R" => {
                if text.is_empty()
                    || text.chars().count() > kentos_commands::MAX_REVISION_NUMBER
                    || text.chars().any(char::is_control)
                {
                    cx.say(Level::Warn, "Revizyonun numarası boş olmayan, en çok 16 karakter olmalı (1, A, B2 gibi).");
                    return false;
                }
                // A revision the drawing has: its date and description come with it.
                (self.date, self.description) = revisions::data_of(cx.doc, text);
                self.mark = Some(Some(text.to_owned()));
            }
            "T" | "A" => {
                let (most, what) = if key == "T" {
                    (kentos_commands::MAX_REVISION_DATE, "tarihi")
                } else {
                    (kentos_commands::MAX_REVISION_DESCRIPTION, "açıklaması")
                };
                if text.chars().count() > most || text.chars().any(char::is_control) {
                    cx.say(Level::Warn, format!("Revizyonun {what} en çok {most} karakter, tek satır olmalı."));
                    return false;
                }
                let value = (!text.is_empty()).then(|| text.to_owned());
                if key == "T" {
                    self.date = value;
                } else {
                    self.description = value;
                }
            }
            _ => return false,
        }
        self.see_tag(cx);
        true
    }

    fn preview(&self, _format: &Format) -> Preview {
        let pts: Vec<Vec2> = self.d.pts.iter().copied().chain(self.d.hover).collect();
        let tracking = self.d.hover.and(self.d.tracking);
        if pts.len() < 2 {
            return Preview {
                tracking,
                ..Preview::default()
            };
        }
        let input = RevcloudCreate {
            layer_id: String::new(),
            frame: self.frame_of(&pts),
            arc: self.arc_length(),
            color: None,
            line_weight: None,
            line_type: None,
            attrs: None,
            label: None,
            symbol: None,
            expected_revision: None,
            rules: None,
        };
        let stroke = match shapes::revcloud_ring(None, &input) {
            Ok((cloud, bulges)) => Stroke::solid(
                bulge_path_outline(&cloud, bulges.as_deref(), true, DEFAULT_STEP),
                true,
            )
            .width(1.5),
            Err(_) => Stroke::dashed(pts, false, [4.0, 3.0]),
        };
        let mut strokes = vec![stroke];
        strokes.extend(self.tag.iter().cloned());
        Preview {
            strokes,
            tracking,
            ..Preview::default()
        }
    }
}
