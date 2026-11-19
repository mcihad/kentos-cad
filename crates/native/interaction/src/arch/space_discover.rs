//! Review a whole level or a picked region before one atomic room creation.
use crate::edge::Outline;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, OptionChoice, Pointer, Preview, Tag, Tone, Tool};
use crate::tool_settings::{Setting, choices};
use crate::{Format, Level, Vec2, points};
use kentos_commands::*;
use kentos_native_application::{ExecutionContext, spaces, spaces_discover};

pub const ID: &str = "spaceDiscover";
pub const LABEL: &str = "Toplu mahal bul";
const EMPTY_SCOPE: &str = "Bu kapsamda kapalı oda bulunamadı.";
const ALREADY_DEFINED: &str = "Bulunan mahaller zaten tanımlı.";

#[derive(Default)]
pub struct Discover {
    level: crate::space_level::Choice,
    sources: crate::space_sources::Choice,
    method: Option<SpaceMethod>,
    columns: Option<SpaceColumns>,
    region: bool,
    corners: Vec<Vec2>,
    revision: Option<u64>,
    layer: String,
    counts: [usize; 3],
    error: Option<String>,
    drawn: Preview,
}
impl Discover {
    fn request(&self, cx: &Context<'_>) -> SpacesDiscover {
        SpacesDiscover {
            // The spaces' layer (docs/adr/8008).
            layer_id: crate::arch_layer::planned(&crate::arch_layer::Kind::Space, cx),
            sources: self.sources.sources(),
            scope: Some(self.level.scope()),
            level: self.level.id(),
            method: self.method,
            columns: Some(self.columns.unwrap_or(SpaceColumns::Net)),
            bounds: (self.region && self.corners.len() == 2)
                .then(|| [points::wire(self.corners[0]), points::wire(self.corners[1])]),
            attrs: None,
            expected_revision: self.revision.map(|v| v.to_string()),
        }
    }
    fn see(&mut self, cx: &Context<'_>) {
        self.level.see(cx.doc.settings());
        // Etkin kat while it has walls (docs/adr/0713).
        self.level.follow_where_walls(cx.level, cx.doc);
        self.revision = Some(cx.doc.revision());
        self.layer = cx.doc.layers().active().into();
        self.counts = [0; 3];
        self.error = None;
        self.drawn = Preview::default();
        if self.region && self.corners.len() != 2 {
            return;
        }
        let mut input = self.request(cx);
        input.expected_revision = None;
        match spaces_discover::plan_in(cx.doc, &input) {
            CommandResult::Completed { output, .. } => {
                let mut tag = None;
                for p in output.proposals {
                    let (k, tone, dash) = if !p.conflicts.is_empty() {
                        (2, Tone::Danger, None)
                    } else if !p.existing.is_empty() {
                        (1, Tone::Snap, Some([4., 3.]))
                    } else {
                        (0, Tone::Accent, None)
                    };
                    self.counts[k] += 1;
                    tag = tag.or(p.space.seed.map(|p| Vec2::new(p.x, p.y)));
                    let shape = kentos_geometry_core::geom::space::as_polygon(&spaces::areas(
                        &p.space.parts,
                    ));
                    self.drawn
                        .strokes
                        .extend(Outline::of(&shape, dash, 2., tone).strokes);
                }
                let anchor = tag.or_else(|| self.corners.last().copied());
                if let Some(at) = anchor {
                    self.drawn.tag = Some(Tag {
                        at,
                        lines: if self.counts == [0; 3] {
                            vec![EMPTY_SCOPE.into(), "Başka bir bölge seçebilirsiniz.".into()]
                        } else {
                            vec![
                                format!("{} yeni · {} mevcut · {} çakışan",
                                    self.counts[0], self.counts[1], self.counts[2]),
                                self.review_step().into(),
                            ]
                        },
                    });
                    if self.counts[2] > 0 {
                        self.drawn.tag_tone = Tone::Danger;
                    }
                }
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.error = Some(error.message),
            _ => {}
        }
        if let [a, b] = self.corners.as_slice() {
            self.drawn.path = vec![*a, Vec2::new(b.x, a.y), *b, Vec2::new(a.x, b.y), *a];
        }
    }
    fn review_step(&self) -> &'static str {
        if self.error.is_some() {
            "Kaynakları ve seçilen kapsamı kontrol edin."
        } else if self.counts[2] > 0 {
            "Çakışan sınırları düzeltin veya bölgeyi daraltın."
        } else if self.counts[0] > 0 {
            "Yeni mahalleri Enter ile ekleyin."
        } else if self.counts[1] > 0 {
            ALREADY_DEFINED
        } else {
            EMPTY_SCOPE
        }
    }
    fn options(&self, key: &str) -> Vec<OptionChoice> {
        match key {
            "B" => self.sources.choices(),
            "D" => crate::space_tools::column_choices(self.columns),
            "L" => {
                self.level.setting().choices
            }
            "K" => choices(
                &[("all", "Tüm ağ"), ("region", "Bölge seç")],
                if self.region { "region" } else { "all" },
            ),
            "Y" => choices(
                &[
                    ("project", "Projenin"),
                    ("net", "Net (iç yüzler)"),
                    ("centerline", "Duvar ekseni"),
                    ("gross", "Brüt (dış yüzler)"),
                ],
                self.method.map_or("project", SpaceMethod::name),
            ),
            _ => vec![],
        }
    }
}
impl Tool for Discover {
    fn layer_kind(&self, _cx: &Context<'_>) -> Option<crate::arch_layer::Kind> {
        Some(crate::arch_layer::Kind::Space)
    }

    fn id(&self) -> &'static str {
        ID
    }
    fn label(&self) -> &'static str {
        LABEL
    }
    fn cursor(&self) -> Cursor {
        Cursor::Cross
    }
    fn snaps(&self) -> bool {
        false
    }
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.sources.capture(cx);
        self.see(cx);
        Flow::Stay
    }
    /// Kat follows the new active level at once while it has walls (docs/adr/0720 §4).
    fn level_changed(&mut self, cx: &mut Context<'_>) {
        self.see(cx);
    }
    fn point_count(&self) -> usize {
        if self.region { self.corners.len() } else { 1 }
    }
    fn snap_from(&self) -> Option<Vec2> {
        match self.corners.as_slice() {
            [a] => Some(*a),
            _ => None,
        }
    }
    fn prompt(&self) -> Prompt {
        let step = if self.region && self.corners.len() < 2 {
            if self.corners.is_empty() {
                "bölgenin ilk köşesini belirtin"
            } else {
                "bölgenin karşı köşesini belirtin"
            }
        } else {
            self.review_step()
        };
        let prompt = Prompt::new(LABEL, step)
            .option("Kapsam", "K")
            .option("Kat", "L")
            .option("Kaynaklar", "B")
            .option("Alan yöntemi", "Y")
            .option("Kolon alanı", "D");
        if self.error.is_none() && self.counts[0] > 0 && self.counts[2] == 0 {
            prompt.option("Uygula", "Enter")
        } else if self.error.is_none() && self.counts[2] == 0 && (self.counts[1] > 0 || !self.region) {
            prompt.option("Bitir", "Enter")
        } else {
            prompt
        }
    }
    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.revision != Some(cx.doc.revision()) || self.layer != cx.doc.layers().active() {
            self.see(cx);
        }
        if self.region && self.corners.len() == 1 {
            let a = self.corners[0];
            let b = p.raw;
            self.drawn.path = vec![a, Vec2::new(b.x, a.y), b, Vec2::new(a.x, b.y), a];
        }
    }
    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if !self.accept_point(p.raw, cx) {
            cx.say(Level::Info,"Önizlemeyi kontrol edip Enter ile uygulayın; kapsamı daraltmak için Bölge seç kullanın.");
        }
    }
    fn accepts_points(&self) -> bool {
        self.region
    }
    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        if !self.region {
            return false;
        }
        if let [a] = self.corners.as_slice()
            && (p.x == a.x || p.y == a.y)
        {
            cx.say(Level::Warn, "Karşı köşe için hem X hem Y farklı olmalı.");
            return true;
        }
        if self.corners.len() == 2 {
            self.corners.clear();
        }
        self.corners.push(p);
        self.see(cx);
        if self.corners.len() == 2 {
            if let Some(error) = &self.error {
                cx.say(Level::Warn, error.clone());
            } else {
                cx.say(Level::Info, self.review_step());
            }
        }
        true
    }
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.region && self.corners.len() != 2 {
            cx.say(Level::Info, "Bölgenin iki karşı köşesini seçin.");
            return Flow::Stay;
        }
        if self.revision != Some(cx.doc.revision()) || self.layer != cx.doc.layers().active() {
            self.see(cx);
            cx.say(Level::Info, "Çizim değişti. Güncel mahal önizlemesini kontrol edin.");
            return Flow::Stay;
        }
        if self.error.is_none() && self.counts[0] == 0 && self.counts[2] == 0 {
            cx.say(Level::Info, self.review_step());
            return if self.region && self.counts[1] == 0 { Flow::Stay } else { Flow::Exit };
        }
        let mut written = None;
        let before = cx.doc.revision();
        crate::arch_layer::writing(&crate::arch_layer::Kind::Space, cx, |_, cx| {
            let mut input = self.request(cx);
            input.expected_revision =
                crate::arch_layer::carried(input.expected_revision, before, cx);
            let result = spaces_discover::execute(&mut ExecutionContext::new(cx.doc), input);
            written = points::written(result, cx);
            written.is_some()
        });
        if let Some(done) = written {
            cx.say(
                Level::Success,
                format!(
                    "{} yeni mahal eklendi; mevcut mahaller korundu.",
                    done.uids.len()
                ),
            );
            Flow::Exit
        } else {
            self.see(cx);
            Flow::Stay
        }
    }
    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if self.corners.pop().is_some() {
            self.see(cx);
            true
        } else {
            false
        }
    }
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        match upper_tr(text.trim()).as_str() {
            "B" => {self.sources.toggle();self.see(cx);true}
            "K" => {
                self.region = !self.region;
                self.corners.clear();
                self.see(cx);
                true
            }
            "D" => {
                self.columns=Some(match self.columns.unwrap_or(SpaceColumns::Net) {
                    SpaceColumns::Ignore => SpaceColumns::Net, SpaceColumns::Net => SpaceColumns::All, SpaceColumns::All => SpaceColumns::Ignore,
                });
                self.see(cx);true
            }
            "L" => {
                self.level.next();
                self.see(cx);
                true
            }
            "Y" => {
                self.method = match self.method {
                    None => Some(SpaceMethod::Net),
                    Some(SpaceMethod::Net) => Some(SpaceMethod::Centerline),
                    Some(SpaceMethod::Centerline) => Some(SpaceMethod::Gross),
                    Some(SpaceMethod::Gross) => None,
                };
                self.see(cx);
                true
            }
            _ if self.region => match cx.typed_point(text, self.snap_from(), None) {
                Some(p) => self.accept_point(p, cx),
                None => false,
            },
            _ => false,
        }
    }
    fn settings(&self, _: &Format) -> Vec<Setting> {
        let mut rows = vec![
            Setting::choice("K", "Kapsam", self.options("K")),
            self.level.setting(),
            self.sources.setting(),
            Setting::choice("Y", "Alan yöntemi", self.options("Y")),
            Setting::choice("D", "Kolon alanı", self.options("D")),
            Setting::text("S", "Yeni", self.counts[0].to_string()).read_only(),
            Setting::text("V", "Mevcut", self.counts[1].to_string()).read_only(),
            Setting::text("C", "Çakışan", self.counts[2].to_string()).read_only(),
        ];
        if let Some(error) = &self.error {
            rows.push(Setting::text("E", "Durum", error).read_only());
        }
        rows
    }
    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        self.options(key)
    }
    fn choose_option(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        self.configure(key, value, cx)
    }
    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        let valid = match (key, value) {
            ("B",_) => self.sources.choose(value),
            ("D", _) => {
                if let Some(m) = SpaceColumns::ALL.into_iter().find(|m|m.name()==value) { self.columns=Some(m);true } else { false }
            }
            ("L", _) => {
                self.level.see(cx.doc.settings());
                self.level.choose(value)
            }
            ("K", "all" | "region") => {
                self.region = value == "region";
                self.corners.clear();
                true
            }
            ("Y", "project") => {
                self.method = None;
                true
            }
            ("Y", _) => {
                if let Some(m) = SpaceMethod::ALL.into_iter().find(|m| m.name() == value) {
                    self.method = Some(m);
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        if valid {
            self.see(cx);
        }
        valid
    }
    fn preview(&self, _: &Format) -> Preview {
        self.drawn.clone()
    }
}
