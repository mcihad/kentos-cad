//! Çoklu tara (docs/adr/0186 §4; Netcad's Çoklu Tara; the web's
//! `HatchSelectedTool`, `apps/web/src/tools/hatchSelectedTool.ts`):
//! selection first; each closed object of it (an area and each part of a
//! multi-part one, a circle, a full ellipse, a closed curve) is hatched by
//! itself with Tarama's options, from a point inside it (its seed), its
//! islands left out when Adalar says so, its texts and inserts when Yazılar
//! does, following its objects when İlişkili is on. Enter writes them all
//! in one step, “Tarama”; the regions show outlined before.

use kentos_contracts::{CreateOperation, EntityGeometry};
use kentos_domain::Slot;
use kentos_geometry_core::entity::polygon_ring;
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::hatch_pattern::too_dense;
use kentos_geometry_core::ops::areas::areas_of_entity;
use kentos_geometry_core::ops::hatch_region::{cut_base, pick, seed_of};
use kentos_geometry_core::text::Font;
use kentos_native_application::geometry::{core_pattern, drawing_font};

use crate::Vec2;
use crate::format::Format;
use crate::hatch::{Tie, assoc_of, cutouts_in, islands_in, left_out};
use crate::hatch_options::{self as options, Asking};
use crate::log::Level;
use crate::modify::{Modify, Stages};
use crate::points::{self, wire_all};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{self, Context, Flow, Memory, OptionChoice, Preview};
use kentos_geometry_core::tools::point_text::js_trim;

/// The tool's id: its command is `tool.hatchSelected`.
pub const ID: &str = "hatchSelected";
pub const LABEL: &str = "Çoklu tara";

/// A region to hatch: its part's area cut, the seed it was picked by and what it came from.
struct Planned {
    area: Area,
    seed: Vec2,
    tie: Tie,
}

#[derive(Default)]
pub struct HatchSelected {
    /// The selection's objects, in its order.
    slots: Vec<Slot>,
    asking: Option<Asking>,
    /// The regions for the drawing and the options as they are.
    plan: Vec<Planned>,
    /// What the plan was made for: the drawing's revision and the options.
    planned: Option<(u64, Memory)>,
    memory: Memory,
    plot_scale: f64,
    font: Font,
}

fn ring_points(r: &Ring) -> Vec<Vec2> {
    polygon_ring(&r.pts, r.bulges.as_deref())
}

impl HatchSelected {
    pub fn tool() -> Modify<Self> {
        Modify::with(Self::default())
    }

    /// Every closed object's regions, each part by itself from a point inside it.
    fn plan(&mut self, cx: &Context<'_>) {
        let key = (cx.doc.revision(), self.memory);
        if self.planned == Some(key) {
            return;
        }
        let m = self.memory;
        let mut plan = Vec::new();
        for slot in &self.slots {
            let id = f64::from(slot.0);
            let Some(item) = cx.spatial.store().get(id) else {
                continue;
            };
            for part in areas_of_entity(&item.shape) {
                let Some(seed) = seed_of(&part) else {
                    continue;
                };
                let (islands, island_areas) = if m.hatch_islands {
                    islands_in(id, &part, &item.bounds, cx)
                } else {
                    (Vec::new(), Vec::new())
                };
                let (cutouts, boxes) = if m.hatch_texts {
                    cutouts_in(&item.bounds, self.font, cx)
                } else {
                    (Vec::new(), Vec::new())
                };
                let cut = cut_base(&part, &island_areas, &boxes);
                let Some(area) = pick(&cut.parts, seed).cloned() else {
                    continue;
                };
                let tie = Tie {
                    outer: id,
                    islands: cut
                        .islands
                        .iter()
                        .filter_map(|&k| islands.get(k).copied())
                        .collect(),
                    cutouts: cut
                        .cutouts
                        .iter()
                        .filter_map(|&k| cutouts.get(k).copied())
                        .collect(),
                };
                plan.push(Planned { area, seed, tie });
            }
        }
        self.plan = plan;
        self.planned = Some(key);
    }

    /// Writes every region's hatch in one step.
    fn write(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        self.plan(cx);
        let Some(pattern) = options::pattern(&self.memory, self.plot_scale) else {
            return Flow::Exit;
        };
        if self.plan.is_empty() {
            cx.say(Level::Warn, "Çoklu tara: taranacak bölge kalmadı; adalar ya da yazılar alanların hepsini kaplıyor.");
            return Flow::Exit;
        }
        let core = core_pattern(&pattern);
        let mut objects = Vec::with_capacity(self.plan.len());
        let (mut islands, mut texts) = (0, 0);
        for planned in &self.plan {
            let ring = ring_points(&planned.area.outer);
            if too_dense(&ring, &core) {
                cx.say(
                    Level::Warn,
                    "Desen bu alanlar için çok sık; ölçeği ya da çizim ölçeğini büyütün ya da başka bir desen seçin.",
                );
                return Flow::Stay;
            }
            let holes: Vec<Vec<Vec2>> = planned.area.holes.iter().map(ring_points).collect();
            islands += planned.tie.islands.len();
            texts += planned.tie.cutouts.len();
            let assoc = if self.memory.hatch_assoc {
                assoc_of(&planned.tie, planned.seed, cx)
            } else {
                None
            };
            objects.push(EntityGeometry::Hatch {
                ring: wire_all(&ring),
                holes: (!holes.is_empty()).then(|| holes.iter().map(|h| wire_all(h)).collect()),
                pattern: pattern.clone(),
                assoc,
            });
        }
        let n = objects.len();
        if points::write_objects(objects, Some(CreateOperation::Hatch), cx).is_none() {
            return Flow::Stay;
        }
        let tie = Tie {
            outer: 0.0,
            islands: vec![0.0; islands],
            cutouts: vec![0.0; texts],
        };
        let name = options::choice(&self.memory).name;
        cx.say(
            Level::Success,
            format!(
                "{LABEL}: {n} {name} tarama eklendi{}",
                left_out(&Some(tie), 0)
            ),
        );
        Flow::Exit
    }
}

impl Stages for HatchSelected {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.memory = *cx.memory;
        self.plot_scale = cx.doc.settings().plot_scale;
        self.font = drawing_font(cx.doc.settings().drawing_font);
    }

    /// The selection's closed objects; none: said, and the tool leaves.
    fn begin(&mut self, cx: &mut Context<'_>) -> Flow {
        self.asking = None;
        self.planned = None;
        self.see(cx);
        let store = cx.spatial.store();
        self.slots = cx
            .selection
            .ids()
            .iter()
            .copied()
            .filter(|s| {
                store
                    .get(f64::from(s.0))
                    .is_some_and(|it| !areas_of_entity(&it.shape).is_empty())
            })
            .collect();
        self.slots.sort_unstable();
        if self.slots.is_empty() {
            cx.say(
                Level::Warn,
                "Çoklu tara: seçimde kapalı nesne yok; kapalı alan, daire, tam elips ya da kapalı eğri seçin.",
            );
            return Flow::Exit;
        }
        self.plan(cx);
        Flow::Stay
    }

    /// The options while picking too.
    fn picking_hint(&self, prompt: Prompt) -> Prompt {
        options::with_region_options(
            options::with_pattern_options(prompt, &self.memory),
            &self.memory,
            true,
        )
    }

    fn picking_prompt(&self, _n: usize) -> Option<Prompt> {
        self.asking.map(|a| options::asking_prompt(LABEL, a))
    }

    fn picking_input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let taken = self.option_text(text, cx);
        self.see(cx);
        taken
    }

    fn anchor(&self) -> Option<Vec2> {
        None
    }

    /// Nothing is placed by the cursor.
    fn snaps(&self) -> bool {
        false
    }

    fn prompt(&self, _n: usize) -> Prompt {
        if let Some(a) = self.asking {
            return options::asking_prompt(LABEL, a);
        }
        let n = self.plan.len();
        let prompt = Prompt::new(
            LABEL,
            format!("{n} bölge taranacak; Enter ya da sağ tıkla tarayın"),
        );
        options::with_region_options(
            options::with_pattern_options(prompt, &self.memory),
            &self.memory,
            true,
        )
    }

    fn point(&mut self, _p: Vec2, _cx: &mut Context<'_>) -> Flow {
        Flow::Stay
    }

    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> Option<Flow> {
        let taken = self.option_text(text, cx);
        self.see(cx);
        self.plan(cx);
        taken.then_some(Flow::Stay)
    }

    fn typed_points(&self) -> bool {
        false
    }

    fn takes_points(&self) -> bool {
        false
    }

    /// Enter writes; while a value is asked it keeps the old one.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.asking.take().is_some() {
            self.see(cx);
            return Flow::Stay;
        }
        self.write(cx)
    }

    /// Esc while a value is asked keeps the old one.
    fn back(&mut self, cx: &mut Context<'_>) -> bool {
        if self.asking.take().is_some() {
            self.see(cx);
            return true;
        }
        false
    }

    fn preview(&self, _hover: Vec2) -> Option<kentos_geometry_core::geom::affine::Affine> {
        None
    }

    /// The regions to hatch, outlined dashed.
    fn stage_preview(&self, _hover: Option<Vec2>, _format: &Format) -> Option<Preview> {
        Some(Preview {
            areas: self
                .plan
                .iter()
                .map(|p| tool::Area {
                    rings: std::iter::once(&p.area.outer)
                        .chain(&p.area.holes)
                        .map(ring_points)
                        .collect(),
                    fill: 0.12,
                    width: 1.5,
                    dash: Some([4.0, 3.0]),
                    fill_tone: tool::Tone::Accent,
                })
                .collect(),
            ..Preview::default()
        })
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        options::choices_of(key, &self.memory)
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        let taken = options::choose(key, typed, &mut self.asking, cx);
        self.see(cx);
        self.plan(cx);
        taken
    }
}

impl HatchSelected {
    /// An option's key, a value asked, or a pattern's name.
    fn option_text(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.asking.is_some() {
            return options::answer(&mut self.asking, text, cx);
        }
        let key = upper_tr(js_trim(text));
        options::option(&key, true, &mut self.asking, cx) || options::typed_choice(text, cx)
    }
}
