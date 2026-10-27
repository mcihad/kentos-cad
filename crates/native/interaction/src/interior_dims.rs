//! İç ölçü zinciri (docs/adr/0711 §5): a cut drawn across walls and columns, the faces it crosses measured as a chain
//! of associative dimensions. The first click starts the cut, the second ends it; the chain then follows the pointer
//! along the cut and a third click puts it there (Enter: on the cut itself). What is shown is
//! `cad.dimensions.interior`'s plan; the write is the command, one undo step. The tool stays for the next chain.

use kentos_commands::{
    CommandResult, DimensionsInterior, Entity, InteriorColumns, InteriorFaces, NewLook,
    STANDARD_STYLE, Vec2 as Point,
};
use kentos_domain::Slot;
use kentos_native_application::{ExecutionContext, dimensions_interior};

use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, OptionChoice, Pointer, Preview, Stroke, Tag, Tone, Tool};
use crate::tool_settings::{Setting, choices};
use crate::{Format, Level, Vec2, points, styles};

pub const ID: &str = "interiorDimensions";
pub const LABEL: &str = "İç ölçü zinciri";

/// A crossing's mark, logical pixels across.
const MARK_PX: f64 = 5.0;

#[derive(Default)]
pub struct InteriorDimensions {
    from: Option<Vec2>,
    to: Option<Vec2>,
    hover: Option<Vec2>,
    skins: bool,
    columns: InteriorColumns,
    total: bool,
    level: Option<String>,
    /// Kat (V) was chosen here: Etkin kat is no longer followed (docs/adr/0713).
    level_chosen: bool,
    /// What Etkin kat gives while it has walls.
    follow: crate::architecture_level::Choice,
    layer: String,
    style: Option<String>,
    layer_choices: Vec<OptionChoice>,
    style_choices: Vec<OptionChoice>,
    level_choices: Vec<OptionChoice>,
    drawn: Preview,
    count: usize,
    curved: usize,
    error: Option<String>,
    written: usize,
}

fn point(p: Vec2) -> Point {
    Point { x: p.x, y: p.y }
}

impl InteriorDimensions {
    fn menus(&mut self, cx: &Context<'_>) {
        self.layer_choices = cx
            .doc
            .layers()
            .leaves()
            .into_iter()
            .filter(|l| {
                !cx.doc.layers().is_locked(&l.id)
                    && l.service.is_none()
                    && l.geometry.is_none_or(|g| g.admits("dimension"))
            })
            .map(|l| OptionChoice {
                typed: l.id.clone(),
                label: cx.doc.layers().path(&l.id),
                checked: l.id == self.layer,
                icon: None,
                preview: None,
                command: None,
            })
            .collect();
        let choice = |typed: &str, label: &str, checked: bool| OptionChoice {
            typed: typed.into(),
            label: label.into(),
            checked,
            icon: None,
            preview: None,
            command: None,
        };
        self.style_choices =
            std::iter::once(choice(STANDARD_STYLE, STANDARD_STYLE, self.style.is_none()))
                .chain(cx.doc.settings().dimension_styles.iter().map(|s| {
                    choice(
                        &s.name,
                        &s.name,
                        self.style.as_deref() == Some(s.name.as_str()),
                    )
                }))
                .collect();
        let levels = cx
            .doc
            .settings()
            .arch
            .as_ref()
            .map(|a| a.levels.clone())
            .unwrap_or_default();
        // Etkin kat while it has walls, until Kat is chosen here (docs/adr/0713).
        if !self.level_chosen {
            self.follow.see(cx.doc.settings());
            self.follow.follow_where_walls(cx.level, cx.doc);
            self.level = self.follow.id();
        }
        self.level_choices = std::iter::once(choice("", "Tüm katlar", self.level.is_none()))
            .chain(
                levels
                    .iter()
                    .map(|l| choice(&l.id, &l.name, self.level.as_deref() == Some(l.id.as_str()))),
            )
            .collect();
    }

    /// The command's input for the cut so far (its end the pointer while it is drawn) and the chain at `at`.
    fn request(&self, at: Option<Vec2>) -> Option<DimensionsInterior> {
        let (from, to) = (self.from?, self.to.or(self.hover)?);
        Some(DimensionsInterior {
            from: point(from),
            to: point(to),
            at: at.map(point),
            faces: Some(if self.skins {
                InteriorFaces::Skins
            } else {
                InteriorFaces::Outer
            }),
            columns: Some(self.columns),
            total: Some(self.total),
            level: self.level.clone(),
            layer_id: self.layer.clone(),
            dimension_style: self.style.clone(),
            height: None,
            mask: None,
            look: NewLook::default(),
            expected_revision: None,
        })
    }

    /// Where the chain goes now: at the pointer once the cut is drawn.
    fn place(&self) -> Option<Vec2> {
        self.to.and(self.hover)
    }

    /// The preview from the command's plan: the cut, the chain and its crossings, or why none forms.
    fn see(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        self.count = 0;
        self.curved = 0;
        self.error = None;
        let Some(input) = self.request(self.place()) else {
            return;
        };
        let (from, to) = (
            Vec2::new(input.from.x, input.from.y),
            Vec2::new(input.to.x, input.to.y),
        );
        let mut cut = Stroke::solid(vec![from, to], false);
        cut.dash = Some([6.0, 4.0]);
        cut.tone = Tone::Snap;
        self.drawn.strokes.push(cut);
        match dimensions_interior::plan_in(cx.doc, &input) {
            CommandResult::Completed { output, warnings } => {
                let mark = cx.view.world_length(MARK_PX);
                let length = (to.x - from.x).hypot(to.y - from.y).max(f64::MIN_POSITIVE);
                let (ux, uy) = ((to.x - from.x) / length, (to.y - from.y) / length);
                for e in &output.entities {
                    let own = crate::wall_dimensions::dimension_preview(
                        e,
                        &cx.format(),
                        cx.doc.settings().drawing_font,
                    );
                    self.drawn.strokes.extend(own.strokes);
                    self.drawn.texts.extend(own.texts);
                    if let Entity::Dimension(d) = e {
                        for p in [d.a, d.b] {
                            // The face met, marked across the cut.
                            let mut tick = Stroke::solid(
                                vec![
                                    Vec2::new(p.x - uy * mark, p.y + ux * mark),
                                    Vec2::new(p.x + uy * mark, p.y - ux * mark),
                                ],
                                false,
                            );
                            tick.tone = Tone::Snap;
                            tick.width = 2.0;
                            self.drawn.strokes.push(tick);
                        }
                    }
                }
                self.count = output.entities.len();
                self.curved = warnings
                    .iter()
                    .filter(|w| w.code == "arch_curved_face_skipped")
                    .count();
                if let Some(w) = warnings
                    .iter()
                    .find(|w| w.code == "arch_curved_face_skipped")
                {
                    self.error = Some(w.message.clone());
                }
                let at = self.place().unwrap_or(to);
                self.drawn.tag = Some(Tag {
                    at,
                    lines: vec![format!("{} ölçü", self.count)],
                });
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.error = Some(error.message);
            }
            _ => {}
        }
    }

    /// Writes the chain at `at` (none: on the cut) and starts over for the next one.
    fn write(&mut self, at: Option<Vec2>, cx: &mut Context<'_>) -> bool {
        let Some(input) = self.request(at) else {
            return false;
        };
        let result = dimensions_interior::execute(&mut ExecutionContext::new(cx.doc), input);
        match points::written(result, cx) {
            Some(output) => {
                let ids: Vec<Slot> = output.ids.iter().map(|&id| Slot(id)).collect();
                let n = ids.len();
                if !ids.is_empty() {
                    cx.selection.set(ids);
                }
                cx.say(
                    Level::Success,
                    format!(
                        "{n} ölçülük iç ölçü zinciri yazıldı; duvarlar kayınca değerler güncellenir, zincir yerinde kalır."
                    ),
                );
                self.written += 1;
                self.from = None;
                self.to = None;
                self.hover = None;
                self.see(cx);
                true
            }
            None => {
                self.see(cx);
                false
            }
        }
    }

    /// A point given (clicked, typed or computed): the cut's start, its end, then the chain's place.
    fn take(&mut self, at: Vec2, cx: &mut Context<'_>) {
        match (self.from, self.to) {
            (None, _) => {
                self.from = Some(at);
                self.hover = Some(at);
            }
            (Some(from), None) => {
                if (at.x - from.x).hypot(at.y - from.y) < 1e-3 {
                    cx.say(
                        Level::Warn,
                        "Kesen çizgi çok kısa; ölçülecek duvarların içinden geçen bir çizgi çizin.",
                    );
                    return;
                }
                self.to = Some(at);
                self.see(cx);
                if self.count == 0 {
                    // Nothing to place: say why and let the cut be drawn again.
                    if let Some(why) = self.error.clone() {
                        cx.say(Level::Warn, why);
                    }
                    self.to = None;
                    self.see(cx);
                }
            }
            (Some(_), Some(_)) => {
                self.hover = Some(at);
                self.write(Some(at), cx);
            }
        }
    }

    fn remember(&self, cx: &mut Context<'_>) {
        cx.memory.interior_skins = self.skins;
        cx.memory.interior_columns = match self.columns {
            InteriorColumns::Ends => 0,
            InteriorColumns::Centers => 1,
            InteriorColumns::None => 2,
        };
        cx.memory.interior_total = self.total;
    }
}

impl Tool for InteriorDimensions {
    fn id(&self) -> &'static str {
        ID
    }
    fn label(&self) -> &'static str {
        LABEL
    }
    fn point_count(&self) -> usize {
        usize::from(self.from.is_some()) + usize::from(self.to.is_some())
    }
    /// The cut's start, for Orto and the snaps that need a last point; none while the chain is placed.
    fn snap_from(&self) -> Option<Vec2> {
        self.from.filter(|_| self.to.is_none())
    }
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.layer = cx.doc.layers().active().into();
        self.style = styles::dimension_style(cx.memory, cx.doc.settings()).map(|s| s.name.clone());
        self.skins = cx.memory.interior_skins;
        self.columns = match cx.memory.interior_columns {
            1 => InteriorColumns::Centers,
            2 => InteriorColumns::None,
            _ => InteriorColumns::Ends,
        };
        self.total = cx.memory.interior_total;
        self.menus(cx);
        if !self.layer_choices.iter().any(|c| c.typed == self.layer)
            && let Some(c) = self.layer_choices.first()
        {
            self.layer = c.typed.clone();
            self.menus(cx);
        }
        Flow::Stay
    }
    fn prompt(&self) -> Prompt {
        match (self.from, self.to) {
            (None, _) => Prompt::new(
                LABEL,
                "kesen çizginin başlangıcını tıklayın (duvarların dışından)",
            ),
            (Some(_), None) => Prompt::new(
                LABEL,
                "kesen çizginin sonunu tıklayın; ölçülecek duvarları ve kolonları kessin",
            ),
            (Some(_), Some(_)) => Prompt::new(LABEL, "zincirin yerini tıklayın")
                .option("Kesen çizgide", "Enter")
                .option("Geri", "Esc"),
        }
    }
    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let at = if self.to.is_some() {
            p.raw
        } else {
            points::constrain(self.from, p, cx).0
        };
        if self.hover != Some(at) {
            self.hover = Some(at);
            if self.from.is_some() {
                self.see(cx);
            }
        }
    }
    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let at = if self.to.is_some() {
            p.raw
        } else {
            points::constrain(self.from, p, cx).0
        };
        self.take(at, cx);
    }
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = upper_tr(text.trim());
        if key == "G" {
            return self.undo_step(cx);
        }
        if self.to.is_some() {
            return false;
        }
        match cx.typed_point(text, self.from, self.hover) {
            Some(at) => {
                self.take(at, cx);
                true
            }
            None => false,
        }
    }
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        match (self.from, self.to) {
            (Some(_), Some(_)) => {
                self.write(None, cx);
                Flow::Stay
            }
            (Some(_), None) => Flow::Stay,
            (None, _) => Flow::Exit,
        }
    }
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        self.undo_step(cx)
    }
    /// One step back: the chain's place, then the cut's end, then its start.
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if self.to.take().is_some() {
            self.see(cx);
            return true;
        }
        if self.from.take().is_some() {
            self.see(cx);
            return true;
        }
        false
    }
    fn preview(&self, _: &Format) -> Preview {
        self.drawn.clone()
    }
    fn settings(&self, _: &Format) -> Vec<Setting> {
        let state = match (self.from, self.to) {
            (None, _) => "Kesen çizginin başlangıcını tıklayın".to_owned(),
            _ if self.count > 0 => format!("{} ölçü", self.count),
            _ => "Yüz kesmiyor".to_owned(),
        };
        let mut rows = vec![
            Setting::text("C", "Zincir", state).read_only(),
            Setting::choice("L", "Ölçü katmanı", self.layer_choices.clone()),
            Setting::choice("S", "Ölçü stili", self.style_choices.clone()),
            Setting::choice(
                "F",
                "Duvarlarda",
                choices(
                    &[("outer", "Dış yüzler"), ("skins", "Bütün katmanlar")],
                    if self.skins { "skins" } else { "outer" },
                ),
            ),
            Setting::choice(
                "K",
                "Kolonlarda",
                choices(
                    &[
                        ("ends", "Kenarlar"),
                        ("centers", "Merkezler"),
                        ("none", "Ölçme"),
                    ],
                    match self.columns {
                        InteriorColumns::Ends => "ends",
                        InteriorColumns::Centers => "centers",
                        InteriorColumns::None => "none",
                    },
                ),
            ),
            Setting::toggle("T", "Toplam ölçü", self.total),
            Setting::choice("V", "Kat", self.level_choices.clone()),
            Setting::hint(
                "H",
                LABEL,
                "Duvarları ve kolonları kesen bir çizgi çizin: kestiği yüzler sırayla ölçülür. Zincirin yerini tıklayın ya da Enter ile kesen çizginin üstüne koyun. Ölçüler yüzlere bağlıdır: duvar kayınca değer değişir, uzayıp kısalınca zincir yerinde kalır.",
            ),
        ];
        if let Some(why) = &self.error {
            rows.push(Setting::notice(
                "E",
                if self.count > 0 {
                    "Not"
                } else {
                    "Ölçülemiyor"
                },
                why,
            ));
        }
        rows
    }
    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        let value = value.trim();
        let valid = match key {
            "L" if self.layer_choices.iter().any(|c| c.typed == value) => {
                self.layer = value.into();
                true
            }
            "S" if self.style_choices.iter().any(|c| c.typed == value) => {
                self.style = (value != STANDARD_STYLE).then(|| value.into());
                styles::take_dimension(value, cx)
            }
            "F" if matches!(value, "outer" | "skins") => {
                self.skins = value == "skins";
                true
            }
            "K" => match value {
                "ends" => {
                    self.columns = InteriorColumns::Ends;
                    true
                }
                "centers" => {
                    self.columns = InteriorColumns::Centers;
                    true
                }
                "none" => {
                    self.columns = InteriorColumns::None;
                    true
                }
                _ => false,
            },
            "T" if matches!(value, "yes" | "no") => {
                self.total = value == "yes";
                true
            }
            "V" if self.level_choices.iter().any(|c| c.typed == value) => {
                self.level = (!value.is_empty()).then(|| value.into());
                self.level_chosen = true;
                true
            }
            _ => false,
        };
        if valid {
            self.remember(cx);
            self.menus(cx);
            self.see(cx);
        }
        valid
    }
    fn finished(&self) -> bool {
        false
    }
}
