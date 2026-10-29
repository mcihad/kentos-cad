//! Etiket, Hepsini etiketle and Çizelge (docs/adr/0344 §5, plan/cad.md `CAD-ARCH-20`–`22`). The tools say what was
//! clicked and chosen; `cad.tags.create` and `cad.schedules.create` compute and write, one undo step each, and their plan
//! is the preview.
//!
//! - Etiket: a click on an opening, a space, a column, a grid, a wall, a stair, a ramp or a block, then where its tag
//!   goes (Enter: beside it); Şablon (Ş), Biçim (B), Kol (K).
//! - Hepsini etiketle: Açıklık (A), Mahal (M), Kolon (K), Aks (S), Duvar (D) chosen, Enter tags every shown one of them
//!   without a tag.
//! - Çizelge: Tür (T), Grup (G), Toplam (O), Sırala (S); the schedule hangs from the cursor by its top left corner, a
//!   click places it.

use kentos_commands::{
    CommandResult, Entity, NewTag, ScheduleOf, ScheduleSort, SchedulesCreate, TagAll, TagShape,
    TaggedKind, TagsCreate,
};
use kentos_domain::Slot;
use kentos_geometry_core::geom::table::layout_of;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::{drawing_font, shape};
use kentos_native_application::{ExecutionContext, schedules_create, tags_create};

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{
    Area, CellGhost, Context, Cursor, Flow, OptionChoice, Pointer, Preview, Stroke, Tag, TextGhost, Tone,
    Tool,
};
use crate::tool_settings::Setting;

#[path = "tag_settings.rs"]
mod settings;

pub const TAG_ID: &str = "tag";
pub const TAG_ALL_ID: &str = "tagAll";
pub const SCHEDULE_ID: &str = "scheduleOpenings";
pub const SCHEDULE_SPACES_ID: &str = "scheduleSpaces";
/// Mahal kaplama çizelgesi and Donatı çizelgesi (docs/adr/0733 §2, §3).
pub const SCHEDULE_FINISHES_ID: &str = "scheduleFinishes";
pub const SCHEDULE_FIXTURES_ID: &str = "scheduleFixtures";

/// The object a tag can be put on under the cursor.
fn taggable_at(at: Vec2, cx: &Context<'_>) -> Option<Slot> {
    cx.spatial
        .hits(at, cx.pick_tolerance())
        .into_iter()
        .find(|s| {
            cx.doc
                .get(*s)
                .is_some_and(|e| TaggedKind::of_kind(e.kind()).is_some())
        })
}

/// A tag drawn as the preview: its frame and leader as strokes, its words as ghost texts.
fn tag_drawn(e: &Entity, scale: f64, out: &mut Preview) {
    let core = shape(e);
    for piece in kentos_geometry_core::geom::arch::pieces(&core, scale) {
        match &piece.shape {
            kentos_geometry_core::entity::Shape::Text {
                p, text, height, ..
            } => out.texts.push(TextGhost {
                p: *p,
                text: text.clone(),
                height: *height,
                rotation: 0.0,
                align: Some(kentos_geometry_core::text::TextAlign::MiddleCenter),
                mask: false,
                face: Default::default(),
                width_factor: 1.0,
            }),
            other => out
                .strokes
                .extend(Outline::of(other, None, 1.5, Tone::Accent).strokes),
        }
    }
}

fn plan_words<P>(result: CommandResult<P>) -> Result<P, String> {
    match result {
        CommandResult::Completed { output, .. } => Ok(output),
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => Err(error.message),
        CommandResult::Queued { .. } | CommandResult::Cancelled => Err(String::new()),
    }
}

// ── Etiket ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// What the Etiket tool waits for after its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Content,
}

/// Etiket.
#[derive(Clone, Debug, Default)]
pub struct TagTool {
    target: Option<Slot>,
    content: Option<String>,
    shape: Option<TagShape>,
    size: Option<f64>,
    leader: bool,
    ask: Option<Ask>,
    hover: Option<Vec2>,
    drawn: Preview,
    problem: Option<String>,
}

impl TagTool {
    pub fn new() -> Self {
        Self::default()
    }

    fn made(&self, at: Option<Vec2>, cx: &Context<'_>) -> Option<TagsCreate> {
        let of = cx.doc.uid(self.target?)?.to_string();
        Some(TagsCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            tags: vec![NewTag {
                of,
                at: at.map(points::wire),
                content: self.content.clone(),
                shape: self.shape,
                size: self.size,
                leader: self.leader.then_some(true),
            }],
            all: None,
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            expected_revision: None,
        })
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        self.problem = None;
        let Some(input) = self.made(self.hover, cx) else {
            return;
        };
        match plan_words(tags_create::plan_in(cx.doc, &input)) {
            Ok(plan) => {
                for e in &plan.tags {
                    tag_drawn(e, cx.doc.settings().plot_scale, &mut self.drawn);
                }
            }
            Err(why) => self.problem = Some(why),
        }
    }

    fn write(&mut self, at: Option<Vec2>, cx: &mut Context<'_>) {
        let Some(input) = self.made(at, cx) else {
            return;
        };
        let result = tags_create::execute(&mut ExecutionContext::new(cx.doc), input);
        said_after(result, cx, |_| {
            "Etiket yazıldı; nesnesi değişince etiketi de değişir.".into()
        });
        self.target = None;
        self.drawn = Preview::default();
    }
}

impl Tool for TagTool {
    fn id(&self) -> &'static str {
        TAG_ID
    }

    fn label(&self) -> &'static str {
        "Etiket"
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

    fn point_count(&self) -> usize {
        usize::from(self.target.is_some())
    }

    fn prompt(&self) -> Prompt {
        if self.ask == Some(Ask::Content) {
            return Prompt::new(
                "Etiket",
                "şablonu yazın ({number}, {type}, {name}, {area} … ; boş: türün şablonu)",
            )
            .option("Geri", "G");
        }
        let shape = self.shape.map_or("Türe göre", TagShape::label);
        let step = if self.target.is_none() {
            "etiketlenecek nesneye tıklayın (açıklık, mahal, kolon, aks, duvar, merdiven, rampa, blok)"
        } else {
            "etiketin yerine tıklayın, Enter ile nesnenin yanına"
        };
        Prompt::new("Etiket", step)
            .option_with(
                "İçerik",
                "İ",
                settings::content_label(self.content.as_deref(), false).to_owned(),
            )
            .option_with(
                "Şablon",
                "Ş",
                if self.content.is_some() { "Özel" } else { "Türün" }.to_owned(),
            )
            .option_with("Biçim", "B", shape.to_owned())
            .option_with(
                "Kol",
                "K",
                if self.leader { "Var" } else { "Yok" }.to_owned(),
            )
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.raw);
        if self.target.is_none() {
            cx.selection.set_hover(taggable_at(p.raw, cx));
        } else {
            self.hover = Some(p.world);
            self.see(cx);
        }
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.target.is_none() {
            self.target = taggable_at(p.raw, cx);
            if self.target.is_none() {
                cx.say(Level::Info, "Burada etiketlenecek nesne yok.");
            }
            self.see(cx);
            return;
        }
        self.write(Some(p.world), cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = upper_tr(js_trim(text));
        if self.ask == Some(Ask::Content) {
            if key == "G" {
                self.ask = None;
            } else {
                self.configure_tag("Ş", text, cx);
            }
            self.see(cx);
            return true;
        }
        match key.as_str() {
            "İ" | "I" => {
                settings::next_content(&mut self.content);
                self.see(cx);
            }
            "Ş" | "S" => self.ask = Some(Ask::Content),
            "B" => {
                self.shape = match self.shape {
                    None => Some(TagShape::Circle),
                    Some(TagShape::None) => None,
                    Some(s) => TagShape::ALL
                        .iter()
                        .skip_while(|x| **x != s)
                        .nth(1)
                        .copied(),
                };
                self.see(cx);
            }
            "K" => {
                self.leader = !self.leader;
                self.see(cx);
            }
            _ => {
                if self.target.is_some()
                    && let Some(p) = cx.typed_point(text, None, self.hover)
                {
                    self.write(Some(p), cx);
                    return true;
                }
                return false;
            }
        }
        true
    }

    fn takes_words(&self) -> bool {
        self.ask.is_some()
    }

    fn settings(&self, _: &Format) -> Vec<Setting> {
        self.tag_rows()
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        self.configure_tag(key, value, cx)
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        self.tag_choices(key)
    }

    fn choose_option(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        self.configure_tag(key, value, cx)
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.ask.is_some() {
            self.ask = None;
            return true;
        }
        if self.target.take().is_some() {
            self.drawn = Preview::default();
            self.see(cx);
            return true;
        }
        false
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.cancel(cx)
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.target.is_none() {
            return Flow::Exit;
        }
        self.write(None, cx);
        Flow::Stay
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = self.drawn.clone();
        if let (Some(why), Some(at)) = (&self.problem, self.hover)
            && !why.is_empty()
        {
            out.tag = Some(Tag {
                at,
                lines: vec![why.clone()],
            });
            out.tag_tone = Tone::Danger;
        }
        out
    }
}

// ── Hepsini etiketle ───────────────────────────────────────────────────────────────────────────────────────────

/// Hepsini etiketle: a read-only plan until one explicit confirmation.
#[derive(Clone, Debug)]
pub struct TagAllTool {
    kinds: Vec<TaggedKind>,
    refresh: bool,
    source: crate::space_level::Choice,
    content: Option<String>,
    shape: Option<TagShape>,
    size: Option<f64>,
    drawn: Preview,
    problem: Option<String>,
    summary: String,
    revision: Option<u64>,
    layer: String,
    layer_label: String,
}

impl Default for TagAllTool {
    fn default() -> Self {
        Self {
            kinds: vec![TaggedKind::Opening, TaggedKind::Space],
            refresh: false,
            source: Default::default(),
            content: None,
            shape: None,
            size: None,
            drawn: Preview::default(),
            problem: None,
            summary: String::new(),
            revision: None,
            layer: String::new(),
            layer_label: String::new(),
        }
    }
}

impl TagAllTool {
    pub fn new() -> Self {
        Self::default()
    }
    fn made(&self, cx: &Context<'_>) -> TagsCreate {
        TagsCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            tags: vec![],
            all: Some(TagAll {
                kinds: self.kinds.clone(),
                refresh: self.refresh.then_some(true),
                layers: None,
                levels: if self.source.scope() == kentos_commands::SpaceScope::Unassigned {
                    Some(vec![None])
                } else {
                    self.source.id().map(|id| vec![Some(id)])
                },
                content: self.content.clone(),
                shape: self.shape,
                size: self.size,
                phases: None,
            }),
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            expected_revision: None,
        }
    }
    fn see(&mut self, cx: &Context<'_>) {
        self.source.see(cx.doc.settings());
        self.revision = Some(cx.doc.revision());
        self.layer = cx.doc.layers().active().to_owned();
        self.layer_label = cx
            .doc
            .layers()
            .get(&self.layer)
            .map_or_else(|| self.layer.clone(), |l| l.name.clone());
        self.drawn = Preview::default();
        self.problem = None;
        match plan_words(tags_create::plan_in(cx.doc, &self.made(cx))) {
            Ok(plan) => {
                let count = plan.tags.len() + plan.refreshed.len();
                self.summary = format!(
                    "{} yeni · {} yenilenecek",
                    plan.tags.len(),
                    plan.refreshed.len()
                );
                // Tags with no words yet (doors without numbers): said before they are written.
                let empty = plan
                    .tags
                    .iter()
                    .chain(&plan.refreshed)
                    .filter(|e| matches!(e, kentos_commands::Entity::Tag(t) if t.tag.text.is_empty()))
                    .count();
                if empty > 0 {
                    self.summary
                        .push_str(&format!(" · {empty} boş (numara verilince dolar)"));
                }
                if count > 200 {
                    self.summary.push_str(" · çizimde ilk 200 gösteriliyor");
                }
                for e in plan.tags.iter().chain(&plan.refreshed).take(200) {
                    tag_drawn(e, cx.doc.settings().plot_scale, &mut self.drawn);
                }
            }
            Err(why) => {
                self.summary = "Yazılacak etiket yok".into();
                self.problem = Some(why);
            }
        }
    }
    fn flip(&mut self, k: TaggedKind) {
        if let Some(i) = self.kinds.iter().position(|x| *x == k) {
            self.kinds.remove(i);
        } else {
            self.kinds.push(k);
            self.kinds.sort();
        }
    }
    fn choices(&self, key: &str) -> Vec<OptionChoice> {
        use crate::tool_settings::choices;
        if let Some((_, k)) = ALL_KEYS.iter().find(|(s, _)| *s == key) {
            return choices(
                &[("yes", "Açık"), ("no", "Kapalı")],
                if self.kinds.contains(k) { "yes" } else { "no" },
            );
        }
        match key {
            "İ" => settings::content_choices(self.content.as_deref(), true),
            "F" => self.source.setting().choices,
            "E" => choices(
                &[("keep", "Koru"), ("refresh", "Yenile")],
                if self.refresh { "refresh" } else { "keep" },
            ),
            "B" => {
                let mut items = vec![("default", "Mevcut / Tür")];
                items.extend(TagShape::ALL.iter().map(|s| (s.name(), s.label())));
                choices(&items, self.shape.map_or("default", TagShape::name))
            }
            _ => vec![],
        }
    }
    fn configure_all(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        if let Some((_, k)) = ALL_KEYS.iter().find(|(s, _)| *s == key) {
            if !matches!(value, "yes" | "no") {
                return false;
            }
            if self.kinds.contains(k) != (value == "yes") {
                self.flip(*k);
            }
        } else {
            match key {
                "İ" => {
                    if !settings::choose_content(&mut self.content, value) {
                        return false;
                    }
                }
                "F" => {
                    self.source.see(cx.doc.settings());
                    if !self.source.choose(value) {
                        return false;
                    }
                }
                "E" if matches!(value, "keep" | "refresh") => self.refresh = value == "refresh",
                "B" => {
                    self.shape = if value == "default" {
                        None
                    } else if let Some(s) = TagShape::ALL.iter().find(|s| s.name() == value) {
                        Some(*s)
                    } else {
                        return false;
                    };
                }
                "Ş" => {
                    let text = value.trim_end_matches(['\r', '\n']).replace("\\n", "\n");
                    if text.trim().is_empty() {
                        self.content = None;
                    } else {
                        if let Err(why) = kentos_commands::tag_template(&text) {
                            cx.say(Level::Warn, why);
                            return false;
                        }
                        self.content = Some(text);
                    }
                }
                "Y" => {
                    if value.trim().is_empty() {
                        self.size = None;
                    } else {
                        let text = value
                            .trim()
                            .strip_suffix("mm")
                            .unwrap_or(value.trim())
                            .trim();
                        let Some(size) =
                            kentos_geometry_core::tools::point_text::parse_number(text)
                                .filter(|v| v.is_finite() && *v > 0. && *v <= 50.)
                        else {
                            cx.say(Level::Warn, "Kağıt üzerinde 0'dan büyük, en çok 50 mm yazı boyu yazın; boş değer mevcudu korur.");
                            return false;
                        };
                        self.size = Some(size);
                    }
                }
                _ => return false,
            }
        }
        self.see(cx);
        true
    }
}

const ALL_KEYS: [(&str, TaggedKind); 8] = [
    ("A", TaggedKind::Opening),
    ("M", TaggedKind::Space),
    ("K", TaggedKind::Column),
    ("S", TaggedKind::Grid),
    ("D", TaggedKind::Wall),
    ("R", TaggedKind::Stair),
    ("P", TaggedKind::Ramp),
    ("O", TaggedKind::Insert),
];

impl Tool for TagAllTool {
    fn id(&self) -> &'static str {
        TAG_ALL_ID
    }
    fn point_count(&self) -> usize {
        0
    }
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }
    fn pointer_move(&mut self, _p: &Pointer, cx: &mut Context<'_>) {
        if self.revision != Some(cx.doc.revision()) || self.layer != cx.doc.layers().active() {
            self.see(cx);
        }
    }
    fn pointer_down(&mut self, _p: &Pointer, _cx: &mut Context<'_>) {}
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }
    fn preview(&self, _format: &Format) -> Preview {
        self.drawn.clone()
    }
    fn label(&self) -> &'static str {
        "Hepsini etiketle"
    }
    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }
    fn snaps(&self) -> bool {
        false
    }
    fn prompt(&self) -> Prompt {
        let mut p = Prompt::new(
            "Hepsini etiketle",
            "ayarları ve önizlemeyi kontrol edin; Enter bir kez uygular",
        );
        p = p.option_with(
            "Mevcut",
            "E",
            if self.refresh { "Yenile" } else { "Koru" }.to_owned(),
        );
        p = p.option_with("Kaynak katı", "F", self.source.label());
        p = p.option_with(
            "İçerik",
            "İ",
            settings::content_label(self.content.as_deref(), true).to_owned(),
        );
        for (key, k) in ALL_KEYS {
            p = p.option_with(
                k.label(),
                key,
                if self.kinds.contains(&k) {
                    "Açık"
                } else {
                    "Kapalı"
                }
                .to_owned(),
            );
        }
        p.option("Uygula", "Enter")
    }
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = upper_tr(js_trim(text));
        if let Some((_, k)) = ALL_KEYS.iter().find(|(s, _)| *s == key) {
            self.flip(*k);
            self.see(cx);
            true
        } else if key == "F" {
            self.source.next();
            self.see(cx);
            true
        } else if key == "E" {
            self.refresh = !self.refresh;
            self.see(cx);
            true
        } else if matches!(key.as_str(), "İ" | "I") {
            settings::next_content(&mut self.content);
            self.see(cx);
            true
        } else {
            false
        }
    }
    fn settings(&self, _: &Format) -> Vec<Setting> {
        let mut source = self.source.setting();
        source.key = "F";
        source.label = "Kaynak katı".into();
        let mut out = vec![
            Setting::choice("E", "Mevcut etiket", self.choices("E")),
            source,
            Setting::notice("N", "Önizleme", &self.summary),
            Setting::text("L", "Etiket katmanı", &self.layer_label).read_only(),
            settings::content_setting(self.content.as_deref(), true),
            Setting::text(
                "Ş",
                "Şablon",
                self.content
                    .as_deref()
                    .unwrap_or_default()
                    .replace('\n', "\\n"),
            ),
            Setting::choice("B", "Çerçeve", self.choices("B")),
            Setting::text(
                "Y",
                "Yazı (mm)",
                self.size.map_or_else(String::new, |s| s.to_string()),
            ),
        ];
        if let Some(why) = &self.problem {
            out.push(Setting::notice("!", "Uygulanamıyor", why));
        }
        out.extend(
            ALL_KEYS
                .iter()
                .map(|(key, k)| Setting::choice(key, k.label(), self.choices(key))),
        );
        out.push(Setting::notice("H", "Kapsam", "Kaynak katı görünür kaynakları süzer; kapı/pencere duvarının katını kullanır. Duvarı kayıp açıklığın katı bilinmez. Yenile yalnız etkin etiket katmanını değiştirir; başka katlar/etiket katmanları ve elle konumlar korunur."));
        out.push(Setting::notice("V", "Boş ayarlar", "Hazır içerik şablon alanını doldurur; alanı ayrıca düzenleyebilirsiniz. Tür / Koru şablonu boşaltır: Yenile'de mevcut içerik korunur, yeni etikette tür varsayılanı kullanılır. Boş boy mevcut değeri korur; \\n yeni satırdır. Kaynak numaraları değişmez."));
        out
    }
    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        self.configure_all(key, value, cx)
    }
    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        self.choices(key)
    }
    fn choose_option(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        self.configure_all(key, value, cx)
    }
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        // Keyboard layer changes need not pass through pointer_move, and the
        // active layer alone does not advance the document revision.
        if self.layer != cx.doc.layers().active() {
            self.see(cx);
            cx.say(
                Level::Warn,
                "Hedef katman değişti; yeni önizlemeyi onaylayın.",
            );
            return Flow::Stay;
        }
        let mut input = self.made(cx);
        input.expected_revision = self.revision.map(|r| r.to_string());
        let result = tags_create::execute(&mut ExecutionContext::new(cx.doc), input);
        let written = said_after(result, cx, |out| {
            format!(
                "{} yeni etiket · {} etiket yenilendi.",
                out.uids.len(),
                out.refreshed.len()
            )
        });
        if written {
            Flow::Exit
        } else {
            self.see(cx);
            Flow::Stay
        }
    }
}

/// What a tag write did, said: its success first, then its warnings (a tag with no words yet, a hidden layer), so the
/// status line ends on what still needs doing (docs/adr/0344's 27 October addendum); a refusal as `points::written`
/// says it. Whether it wrote.
fn said_after(
    result: kentos_commands::CommandResult<kentos_commands::TagsCreated>,
    cx: &mut Context<'_>,
    success: impl FnOnce(&kentos_commands::TagsCreated) -> String,
) -> bool {
    match result {
        kentos_commands::CommandResult::Completed { output, warnings } => {
            cx.say(Level::Success, success(&output));
            for w in warnings {
                cx.say(Level::Warn, w.message);
            }
            true
        }
        other => points::written(other, cx).is_some(),
    }
}

// ── Çizelge ────────────────────────────────────────────────────────────────────────────────────────────────────

/// What the tool places (docs/adr/0733 §5): a schedule's kind, Mahal kaplama a spaces schedule of the finishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchedulePreset {
    Openings,
    Spaces,
    Finishes,
    Columns,
    Walls,
    Fixtures,
}

impl SchedulePreset {
    pub const ALL: [SchedulePreset; 6] = [
        SchedulePreset::Openings,
        SchedulePreset::Spaces,
        SchedulePreset::Finishes,
        SchedulePreset::Columns,
        SchedulePreset::Walls,
        SchedulePreset::Fixtures,
    ];

    pub fn of(self) -> ScheduleOf {
        match self {
            SchedulePreset::Openings => ScheduleOf::Openings,
            SchedulePreset::Spaces | SchedulePreset::Finishes => ScheduleOf::Spaces,
            SchedulePreset::Columns => ScheduleOf::Columns,
            SchedulePreset::Walls => ScheduleOf::Walls,
            SchedulePreset::Fixtures => ScheduleOf::Fixtures,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SchedulePreset::Finishes => "Mahal kaplama çizelgesi",
            other => other.of().label(),
        }
    }

    fn typed(self) -> &'static str {
        match self {
            SchedulePreset::Openings => "openings",
            SchedulePreset::Spaces => "spaces",
            SchedulePreset::Finishes => "finishes",
            SchedulePreset::Columns => "columns",
            SchedulePreset::Walls => "walls",
            SchedulePreset::Fixtures => "fixtures",
        }
    }

    /// Its columns: a finishes schedule's (docs/adr/0733 §3), else the kind's or a grouped one's.
    fn columns(self, group: bool) -> Option<Vec<kentos_commands::ScheduleColumn>> {
        use kentos_commands::ScheduleField::*;
        let fields: &[kentos_commands::ScheduleField] = match (self, group) {
            (SchedulePreset::Finishes, _) => {
                &[Number, Name, FloorFinish, BaseFinish, WallFinish, CeilingFinish, Area, Baseboard]
            }
            (_, false) => return None,
            (SchedulePreset::Openings, true) => &[Type, Form, Width, Height, Count],
            (SchedulePreset::Spaces, true) => &[Usage, Count, Area],
            (SchedulePreset::Columns, true) => &[Type, Width, Count],
            (SchedulePreset::Walls, true) => &[Type, Thickness, Count, Length, Area],
            (SchedulePreset::Fixtures, true) => &[Type, Category, Count],
        };
        Some(
            fields
                .iter()
                .map(|&field| kentos_commands::ScheduleColumn {
                    field,
                    title: None,
                    attr: None,
                })
                .collect(),
        )
    }
}

/// Çizelge: a schedule placed from the cursor; its settings visible (docs/adr/0733 §5).
#[derive(Clone, Debug)]
pub struct ScheduleTool {
    /// What it was started for (its id).
    start: SchedulePreset,
    preset: SchedulePreset,
    group: bool,
    total: bool,
    sorted: bool,
    /// A subtotal after each group of the first column (docs/adr/0733 §4).
    subtotals: bool,
    /// Etkin kat's objects alone.
    active_level: bool,
    /// The plan's table at the origin, made again when an option changes.
    table: Option<kentos_commands::TableEntity>,
    problem: Option<String>,
    hover: Option<Vec2>,
    ghost: Preview,
    /// Placed: the tool leaves, as Tablo ekle's placement does.
    done: bool,
}

impl ScheduleTool {
    pub fn new(of: ScheduleOf) -> Self {
        Self::of_preset(match of {
            ScheduleOf::Openings => SchedulePreset::Openings,
            ScheduleOf::Spaces => SchedulePreset::Spaces,
            ScheduleOf::Columns => SchedulePreset::Columns,
            ScheduleOf::Walls => SchedulePreset::Walls,
            ScheduleOf::Fixtures => SchedulePreset::Fixtures,
        })
    }

    pub fn of_preset(preset: SchedulePreset) -> Self {
        Self {
            start: preset,
            preset,
            // A fixtures schedule counts alike ones on one row.
            group: preset == SchedulePreset::Fixtures,
            total: preset != SchedulePreset::Openings,
            sorted: true,
            subtotals: false,
            active_level: false,
            table: None,
            problem: None,
            hover: None,
            ghost: Preview::default(),
            done: false,
        }
    }

    fn input_at(&self, at: Vec2, cx: &Context<'_>) -> SchedulesCreate {
        let level = cx.level.filter(|_| self.active_level);
        SchedulesCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            at: points::wire(at),
            of: self.preset.of(),
            columns: self.preset.columns(self.group),
            filter: level.map(|l| kentos_commands::ScheduleFilter {
                levels: Some(vec![l.to_owned()]),
                ..Default::default()
            }),
            sort: if self.sorted || self.subtotals {
                vec![ScheduleSort {
                    column: 0,
                    descending: false,
                    header: false,
                    footer: self.subtotals,
                }]
            } else {
                Vec::new()
            },
            group: self.group.then_some(true),
            total: self.total.then_some(true),
            height: None,
            color: cx.draft.color_text(),
            line_weight: cx.draft.line_weight,
            expected_revision: None,
        }
    }

    fn plan(&mut self, cx: &Context<'_>) {
        self.table = None;
        self.problem = None;
        match plan_words(schedules_create::plan_in(
            cx.doc,
            &self.input_at(Vec2::new(0.0, 0.0), cx),
        )) {
            Ok(p) => {
                if let Entity::Table(t) = p.table {
                    self.table = Some(t);
                }
            }
            Err(why) => self.problem = Some(why),
        }
    }

    fn place_ghost(&mut self, cx: &Context<'_>) {
        self.ghost = Preview::default();
        let (Some(h), Some(t)) = (self.hover, self.table.as_ref()) else {
            return;
        };
        let mut t = t.clone();
        t.p = points::wire(h);
        let entity = Entity::Table(t.clone());
        let Some(layout) = layout_of(
            &shape(&entity),
            drawing_font(cx.doc.settings().drawing_font),
        ) else {
            return;
        };
        self.ghost.areas = layout
            .frame
            .iter()
            .map(|strip| Area {
                rings: vec![strip.to_vec()],
                fill: 0.35,
                width: 1.0,
                dash: None,
                fill_tone: Tone::Accent,
            })
            .collect();
        self.ghost.strokes = layout
            .lines
            .iter()
            .map(|[a, b]| Stroke::dashed(vec![*a, *b], false, [4.0, 3.0]))
            .collect();
        self.ghost.cells = layout
            .cells
            .iter()
            .filter_map(|c| {
                let words = t.cells.get(c.row)?.get(c.col)?;
                (!words.is_empty()).then(|| CellGhost {
                    at: c.at,
                    text: words.clone(),
                    height: t.height,
                    bold: c.bold,
                    face: t.face.clone(),
                })
            })
            .collect();
    }

    fn write(&mut self, at: Vec2, cx: &mut Context<'_>) {
        let input = self.input_at(at, cx);
        let result = schedules_create::execute(&mut ExecutionContext::new(cx.doc), input);
        if let Some(out) = points::written(result, cx) {
            self.done = true;
            self.ghost = Preview::default();
            cx.selection.set([Slot(out.id)]);
            cx.say(
                Level::Success,
                format!(
                    "{}: {} satır. Kaynakları değişince satırları da değişir.",
                    self.preset.label(),
                    out.rows
                ),
            );
        }
    }
}

impl Tool for ScheduleTool {
    fn point_count(&self) -> usize {
        0
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn id(&self) -> &'static str {
        match self.start {
            SchedulePreset::Spaces => SCHEDULE_SPACES_ID,
            SchedulePreset::Finishes => SCHEDULE_FINISHES_ID,
            SchedulePreset::Fixtures => SCHEDULE_FIXTURES_ID,
            _ => SCHEDULE_ID,
        }
    }

    fn label(&self) -> &'static str {
        self.preset.label()
    }

    fn settings(&self, _: &Format) -> Vec<Setting> {
        let kinds: Vec<(&str, &str)> = SchedulePreset::ALL.iter().map(|p| (p.typed(), p.label())).collect();
        vec![
            Setting::choice("kind", "Tür", crate::tool_settings::choices(&kinds, self.preset.typed())),
            Setting::choice(
                "level",
                "Kat",
                crate::tool_settings::choices(
                    &[("all", "Bütün katlar"), ("active", "Etkin kat")],
                    if self.active_level { "active" } else { "all" },
                ),
            ),
            Setting::toggle("group", "Aynı satırları birleştir", self.group),
            Setting::toggle("total", "Genel toplam", self.total),
            Setting::toggle("subtotals", "Ara toplam", self.subtotals),
            Setting::toggle("sorted", "Sırala", self.sorted),
        ]
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        let on = value == "yes";
        match key {
            "kind" => match SchedulePreset::ALL.iter().find(|p| p.typed() == value) {
                Some(p) => self.preset = *p,
                None => return false,
            },
            "level" => self.active_level = value == "active",
            "group" => {
                self.group = on;
                self.total = self.total || on;
            }
            "total" => self.total = on,
            "subtotals" => self.subtotals = on,
            "sorted" => self.sorted = on,
            _ => return false,
        }
        self.plan(cx);
        self.place_ghost(cx);
        true
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.plan(cx);
        Flow::Stay
    }

    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.write(p, cx);
        true
    }

    fn prompt(&self) -> Prompt {
        let on = |b: bool| if b { "Açık" } else { "Kapalı" }.to_owned();
        Prompt::new(self.preset.label(), "çizelgenin sol üst köşesine tıklayın")
            .option_with("Tür", "T", self.preset.label().to_owned())
            .option_with("Grup", "G", on(self.group))
            .option_with("Toplam", "O", on(self.total))
            .option_with("Sırala", "S", on(self.sorted))
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        self.place_ghost(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.write(p.world, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        match upper_tr(js_trim(text)).as_str() {
            "T" => {
                let all = SchedulePreset::ALL;
                let i = all.iter().position(|k| *k == self.preset).unwrap_or(0);
                self.preset = all[(i + 1) % all.len()];
            }
            "G" => {
                self.group = !self.group;
                // Alike rows counted: a total says how many in all.
                self.total = self.total || self.group;
            }
            "O" => self.total = !self.total,
            "S" => self.sorted = !self.sorted,
            _ => match cx.typed_point(text, None, self.hover) {
                Some(p) => {
                    self.write(p, cx);
                    return true;
                }
                None => return false,
            },
        }
        self.plan(cx);
        self.place_ghost(cx);
        true
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn finished(&self) -> bool {
        self.done
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = self.ghost.clone();
        if let (Some(why), Some(at)) = (&self.problem, self.hover)
            && !why.is_empty()
        {
            out.tag = Some(Tag {
                at,
                lines: vec![why.clone()],
            });
            out.tag_tone = Tone::Danger;
        }
        out
    }
}
