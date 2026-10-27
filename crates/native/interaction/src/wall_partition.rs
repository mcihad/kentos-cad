//! Measured partition: picks references; native command owns every construction coordinate.
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, Memory, Pointer, Preview, Stroke, Tag};
use crate::tool_settings::{Setting, choices};
use crate::{Format, Level, Tool, Vec2, points, wall};
use kentos_commands::*;
use kentos_native_application::{ExecutionContext, walls_partition};
pub const ID: &str = "wallPartition";
pub const LABEL: &str = "Ölçülü bölme duvar";
#[derive(Default)]
pub struct Partition {
    source: Option<WallEdgeRef>,
    limits: Vec<WallEdgeRef>,
    candidates: Vec<(WallEdgeRef, String)>,
    mode: u8,
    side: PartitionSide,
    measure: PartitionMeasure,
    distance: f64,
    anchor: f64,
    length: f64,
    direction: PartitionDirection,
    ask: Option<String>,
    drawn: Preview,
    memory: Memory,
    styles: Vec<WallStyleDef>,
    hover: Option<Vec2>,
}
impl Partition {
    pub fn new() -> Self {
        Self {
            distance: 3.0,
            length: 2.0,
            ..Self::default()
        }
    }
    fn input_for(&self, cx: &Context<'_>) -> Option<WallsPartition> {
        let style = wall::chosen(cx.memory, cx.doc.settings());
        let fixed = style
            .as_ref()
            .and_then(|s| fixed_width(&s.components))
            .is_some();
        Some(WallsPartition {
            source: self.source.clone()?,
            side: self.side,
            measure: self.measure,
            distance: self.distance,
            extent: match self.mode {
                1 => PartitionExtent::Between {
                    start: self.limits.first()?.clone(),
                    end: self.limits.get(1)?.clone(),
                },
                2 => PartitionExtent::Length {
                    anchor: self.anchor,
                    length: self.length,
                    direction: self.direction,
                },
                _ => PartitionExtent::Source,
            },
            wall: WallsCreate {
                layer_id: cx.doc.layers().active().to_owned(),
                walls: vec![NewWall {
                    pts: vec![],
                    bulges: None,
                    closed: false,
                    width: (!fixed).then_some(cx.memory.wall_width),
                    style: style.map(|s| s.id),
                    justify: Some(WallJustify::Center),
                    offset: None,
                    caps: None,
                    height: None,
                    // Etkin kat (docs/adr/0713).
                    arch: cx.level_arch(),
                    attrs: cx.template_attrs(None),
                    label: cx.template_label(),
                }],
                color: cx.draft.color_text(),
                line_weight: cx.draft.line_weight,
                line_type: cx.draft.line_type,
                expected_revision: None,
            },
        })
    }
    fn see(&mut self, cx: &Context<'_>) {
        self.memory = *cx.memory;
        self.styles = wall::offered(cx.doc.settings());
        self.drawn = Preview::default();
        for r in self.source.iter().chain(self.limits.iter()) {
            if let Some(Entity::Wall(w)) = kentos_domain::Uuid::parse_str(&r.wall)
                .ok()
                .and_then(|u| cx.doc.slot_of(u))
                .and_then(|s| cx.doc.get(s))
                && let (Some(a), Some(b)) = (
                    w.wall.pts.get(r.edge),
                    w.wall.pts.get((r.edge + 1) % w.wall.pts.len()),
                )
            {
                self.drawn.strokes.push(
                    Stroke::solid(vec![Vec2::new(a.x, a.y), Vec2::new(b.x, b.y)], false).width(2.0),
                );
            }
        }
        let Some(input) = self.input_for(cx) else {
            return;
        };
        match walls_partition::plan_in(cx.doc, &input) {
            CommandResult::Completed { output, warnings } => {
                let mut drawn = wall::Drawn::default();
                for w in &output.wall.walls {
                    wall::add(w, &mut drawn);
                }
                self.drawn.areas = drawn.areas;
                self.drawn.strokes.extend(drawn.strokes);
                let map = |p: kentos_commands::Vec2| Vec2::new(p.x, p.y);
                self.drawn.strokes.push(Stroke::dashed(
                    output.source_face.map(map).to_vec(),
                    false,
                    [4.0, 4.0],
                ));
                self.drawn
                    .strokes
                    .push(Stroke::solid(output.dimension.map(map).to_vec(), false));
                self.drawn.tag = Some(Tag {
                    at: map(output.dimension[1]),
                    lines: vec![
                        format!("Net uzaklık: {}", cx.format().length(output.clear_distance)),
                        format!(
                            "Eksen aralığı: {}",
                            cx.format().length(output.axis_distance)
                        ),
                        "Önizleme · Enter veya tıklama ile yerleştir".into(),
                    ],
                });
                if let Some(tag) = &mut self.drawn.tag {
                    tag.lines
                        .extend(warnings.into_iter().take(3).map(|w| w.message));
                }
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.drawn.tag = Some(Tag {
                    at: self.hover.unwrap_or(Vec2::new(0.0, 0.0)),
                    lines: vec![error.message],
                });
            }
            _ => {}
        }
    }
    fn pick(p: &Pointer, cx: &Context<'_>) -> Vec<(WallEdgeRef, String)> {
        // Generic edge tools exclude semantic walls. Use the ordinary visible-geometry aperture;
        // expose every candidate instead of silently searching a broad hidden radius.
        cx.spatial
            .hits(p.raw, cx.pick_tolerance())
            .into_iter()
            .filter_map(|slot| {
                let entity = cx.doc.get(slot)?;
                let Entity::Wall(w) = entity else {
                    return None;
                };
                if !crate::selectable::entity_allowed(cx.draft.select_kinds, entity)
                    || !crate::selectable::on_selected_layer(cx.doc, entity)
                {
                    return None;
                }
                let pts = &w.wall.pts;
                let count = if w.wall.closed {
                    pts.len()
                } else {
                    pts.len().saturating_sub(1)
                };
                let map = |p: kentos_commands::Vec2| Vec2::new(p.x, p.y);
                let edge = (0..count).min_by(|a, b| {
                    kentos_geometry_core::geometry::dist_to_segment(
                        p.raw,
                        map(pts[*a]),
                        map(pts[(*a + 1) % pts.len()]),
                    )
                    .total_cmp(
                        &kentos_geometry_core::geometry::dist_to_segment(
                            p.raw,
                            map(pts[*b]),
                            map(pts[(*b + 1) % pts.len()]),
                        ),
                    )
                })?;
                let layer = cx
                    .doc
                    .layers()
                    .get(&w.base.layer_id)
                    .map_or(w.base.layer_id.as_str(), |l| l.name.as_str());
                Some((
                    WallEdgeRef {
                        wall: cx.doc.uid(slot)?.to_string(),
                        edge,
                    },
                    format!("Duvar {} · {} · kenar {}", w.base.id, layer, edge + 1),
                ))
            })
            .collect()
    }
    fn write(&mut self, cx: &mut Context<'_>) {
        let Some(input) = self.input_for(cx) else {
            cx.say(
                Level::Warn,
                "Önce kaynak duvarı ve gerekiyorsa iki sınır kenarını seçin.",
            );
            return;
        };
        if points::written(
            walls_partition::execute(&mut ExecutionContext::new(cx.doc), input),
            cx,
        )
        .is_some()
        {
            self.source = None;
            self.candidates.clear();
            self.limits.clear();
            cx.say(Level::Success, "Ölçülü bölme duvar eklendi.");
        }
        self.see(cx);
    }
}
impl Tool for Partition {
    fn id(&self) -> &'static str {
        ID
    }
    fn label(&self) -> &'static str {
        LABEL
    }
    fn point_count(&self) -> usize {
        usize::from(self.source.is_some()) + self.limits.len()
    }
    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }
    fn prompt(&self) -> Prompt {
        let step = if self.ask.is_some() {
            "ayar değerini yazın"
        } else if self.source.is_none() {
            "referans alınacak düz duvar kenarını seçin"
        } else if self.mode == 1 && self.limits.is_empty() {
            "bölmenin ulaşacağı ilk sınır duvarının kenarını seçin"
        } else if self.mode == 1 && self.limits.len() == 1 {
            "ikinci sınır duvarının kenarını seçin"
        } else {
            "ayarları ve önizlemeyi kontrol edin; Enter ile yerleştirin"
        };
        let prompt = Prompt::new(LABEL, step)
            .option("Uzaklık", "U")
            .option("Kalınlık", "E")
            .option("Stil", "S")
            .option("Ölçme", "M")
            .option("Yan", "Y")
            .option("Uçlar", "B");
        let prompt = if self.candidates.len() > 1 {
            prompt.option("Kaynak", "C")
        } else {
            prompt
        };
        if self.mode == 2 {
            prompt
                .option("Boy", "L")
                .option("Başlangıç", "A")
                .option("Yön", "D")
        } else {
            prompt
        }
    }
    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.raw);
        self.see(cx)
    }
    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.ask.is_some() {
            return;
        }
        if self.source.is_none() || (self.mode == 1 && self.limits.len() < 2) {
            let found = Self::pick(p, cx);
            if let Some((r, _)) = found.first() {
                if self.source.is_none() {
                    self.source = Some(r.clone());
                    self.candidates = found;
                    if self.candidates.len() > 1 {
                        cx.say(Level::Info,"Birden çok duvar bulundu; araç ayarlarındaki Kaynak duvar listesinden seçimi kontrol edin.");
                    }
                } else {
                    self.limits.push(r.clone());
                }
            } else {
                cx.say(Level::Warn, "Bir duvarın düz kenarına tıklayın.");
            }
            self.see(cx);
        } else {
            self.write(cx);
        }
    }
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if let Some(key) = self.ask.take() {
            return self.configure(&key, text, cx);
        }
        let key = upper_tr(text.trim());
        if self.settings(&cx.format()).iter().any(|s| s.key == key) {
            self.ask = Some(key);
            true
        } else {
            false
        }
    }
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.ask.take().is_some() {
            return Flow::Stay;
        }
        if self.input_for(cx).is_none() {
            return Flow::Exit;
        }
        self.write(cx);
        if self.source.is_none() {
            Flow::Exit
        } else {
            Flow::Stay
        }
    }
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.ask.take().is_some() {
            return true;
        }
        if self.limits.pop().is_some() {
            self.see(cx);
            return true;
        }
        if self.source.take().is_some() {
            self.candidates.clear();
            self.see(cx);
            return true;
        }
        false
    }
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.cancel(cx)
    }
    fn preview(&self, _f: &Format) -> Preview {
        self.drawn.clone()
    }
    fn settings(&self, f: &Format) -> Vec<Setting> {
        let mut rows = crate::tool_settings::wall_rows(&self.memory, &self.styles, f);
        rows.retain(|s| matches!(s.key, "S" | "E"));
        if self.candidates.len() > 1 {
            rows.insert(
                0,
                Setting::choice(
                    "C",
                    "Kaynak duvar",
                    self.candidates
                        .iter()
                        .enumerate()
                        .map(|(i, (r, label))| crate::tool::OptionChoice {
                            typed: i.to_string(),
                            label: label.clone(),
                            icon: None,
                            preview: None,
                            checked: self.source.as_ref() == Some(r),
                            command: None,
                        })
                        .collect(),
                ),
            );
        }
        rows.extend([
            Setting::text("U", "Uzaklık", f.length(self.distance)),
            Setting::choice(
                "M",
                "Uzaklık türü",
                choices(
                    &[("clear", "Net yüzler arası"), ("axis", "Eksenler arası")],
                    if self.measure == PartitionMeasure::Clear {
                        "clear"
                    } else {
                        "axis"
                    },
                ),
            ),
            Setting::choice(
                "Y",
                "Kaynağın yanı",
                choices(
                    &[("left", "Sol"), ("right", "Sağ")],
                    if self.side == PartitionSide::Left {
                        "left"
                    } else {
                        "right"
                    },
                ),
            ),
            Setting::choice(
                "B",
                "Uçlar",
                choices(
                    &[
                        ("source", "Kaynak uçları"),
                        ("between", "İki duvar arasında"),
                        ("length", "Verilen boy"),
                    ],
                    match self.mode {
                        1 => "between",
                        2 => "length",
                        _ => "source",
                    },
                ),
            ),
        ]);
        if self.mode == 2 {
            rows.extend([
                Setting::text("A", "Başlangıç", f.length(self.anchor)),
                Setting::text("L", "Boy", f.length(self.length)),
                Setting::choice(
                    "D",
                    "Yön",
                    choices(
                        &[
                            ("forward", "İleri"),
                            ("backward", "Geri"),
                            ("both", "İki yön"),
                        ],
                        match self.direction {
                            PartitionDirection::Forward => "forward",
                            PartitionDirection::Backward => "backward",
                            PartitionDirection::Both => "both",
                        },
                    ),
                ),
            ]);
        }
        rows
    }
    fn option_choices(&self, key: &str) -> Vec<crate::tool::OptionChoice> {
        self.settings(&Format::default())
            .into_iter()
            .find(|s| s.key == key)
            .map_or_else(Vec::new, |s| s.choices)
    }
    fn choose_option(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        self.configure(key, value, cx)
    }
    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        let canonical = self
            .settings(&cx.format())
            .into_iter()
            .find(|s| s.key == key)
            .and_then(|s| {
                s.choices
                    .into_iter()
                    .find(|c| {
                        upper_tr(&c.label) == upper_tr(value.trim())
                            || upper_tr(&c.typed) == upper_tr(value.trim())
                    })
                    .map(|c| c.typed)
            });
        let value = canonical.as_deref().unwrap_or(value);
        let took = match (key, value) {
            ("C", _) => {
                let Some((r, _)) = value
                    .parse::<usize>()
                    .ok()
                    .and_then(|i| self.candidates.get(i))
                else {
                    return false;
                };
                self.source = Some(r.clone());
                self.limits.clear();
                true
            }
            ("S" | "E", _) => crate::tool_settings::wall_apply(key, value, cx),
            ("M", "clear" | "axis") => {
                self.measure = if value == "clear" {
                    PartitionMeasure::Clear
                } else {
                    PartitionMeasure::Axis
                };
                true
            }
            ("Y", "left" | "right") => {
                self.side = if value == "left" {
                    PartitionSide::Left
                } else {
                    PartitionSide::Right
                };
                true
            }
            ("B", "source" | "between" | "length") => {
                self.mode = match value {
                    "between" => 1,
                    "length" => 2,
                    _ => 0,
                };
                self.limits.clear();
                true
            }
            ("D", "forward" | "backward" | "both") => {
                self.direction = match value {
                    "backward" => PartitionDirection::Backward,
                    "both" => PartitionDirection::Both,
                    _ => PartitionDirection::Forward,
                };
                true
            }
            ("U" | "A" | "L", _) => {
                let Some(v) = cx
                    .typed_length(value)
                    .filter(|v| v.is_finite() && (*v > 0.0 || (key == "A" && *v == 0.0)))
                else {
                    cx.say(
                        Level::Warn,
                        "Pozitif bir uzunluk yazın; başlangıç sıfır olabilir.",
                    );
                    return false;
                };
                match key {
                    "U" => self.distance = v,
                    "A" => self.anchor = v,
                    _ => self.length = v,
                };
                true
            }
            _ => false,
        };
        self.see(cx);
        took
    }
}
