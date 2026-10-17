//! Çoklu tara (docs/adr/0186 §4; Netcad's Çoklu Tara; the web's
//! `HatchSelectedTool`, `apps/web/src/tools/hatchSelectedTool.ts`):
//! selection first; each closed object of it (an area and each part of a
//! multi-part one, a circle, a full ellipse, a closed curve) is hatched by
//! itself with Tarama's options, from a point inside it (its seed), its
//! islands left out when Adalar says so, its texts and inserts when Yazılar
//! does, following its objects when İlişkili is on. The product command
//! `cad.entities.hatch` with its objects does the work (docs/adr/0254 §7):
//! the regions are its own (`regions_in`), shown outlined before; Enter
//! writes them all in one step, “Tarama”.

use kentos_contracts::{EntitiesHatch, HatchRegion, NewLook};
use kentos_domain::Slot;
use kentos_geometry_core::entity::polygon_ring;
use kentos_geometry_core::geom::arrangement::Ring;
use kentos_geometry_core::ops::areas::areas_of_entity;
use kentos_native_application::region::{Planned, RegionCache};
use kentos_native_application::{ExecutionContext, entities_hatch};

use crate::Vec2;
use crate::format::Format;
use crate::hatch::left_out;
use crate::hatch_options::{self as options, Asking};
use crate::log::Level;
use crate::modify::{Modify, Stages};
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{self, Context, Flow, Memory, OptionChoice, Preview};
use kentos_geometry_core::tools::point_text::js_trim;

/// The tool's id: its command is `tool.hatchSelected`.
pub const ID: &str = "hatchSelected";
pub const LABEL: &str = "Çoklu tara";

#[derive(Default)]
pub struct HatchSelected {
    /// The selection's objects, in its order.
    slots: Vec<Slot>,
    asking: Option<Asking>,
    /// The regions for the drawing and the options as they are: the command's.
    plan: Vec<Planned>,
    /// What the command found the regions from, kept while the drawing stays.
    cache: RegionCache,
    memory: Memory,
    /// The project's own patterns' names, as of the last call (docs/adr/0331 §1).
    user: Vec<String>,
}

fn ring_points(r: &Ring) -> Vec<Vec2> {
    polygon_ring(&r.pts, r.bulges.as_deref())
}

impl HatchSelected {
    pub fn tool() -> Modify<Self> {
        Modify::with(Self::default())
    }

    /// The command's input: the selection's closed objects, the options,
    /// the active layer and colour.
    fn request(&self, cx: &Context<'_>) -> EntitiesHatch {
        let mut m = *cx.memory;
        // The pattern's angle is from the UCS's x axis (docs/adr/0261 §4).
        m.hatch_angle += cx.ucs_degrees();
        let doc = &*cx.doc;
        EntitiesHatch {
            region: HatchRegion::Objects {
                uids: self
                    .slots
                    .iter()
                    .filter_map(|s| doc.uid(*s).map(|u| u.to_string()))
                    .collect(),
                islands: Some(m.hatch_islands),
                texts: Some(m.hatch_texts),
                associative: Some(m.hatch_assoc),
                island_style: options::island_style(&m),
            },
            pattern: options::spec(&m, &self.user),
            layer_id: doc.layers().active().to_owned(),
            look: NewLook {
                color: cx.draft.color_text(),
                line_weight: None,
                attrs: cx.template_attrs(None),
                label: cx.template_label(),
                symbol: cx.template_symbol(),
            },
            expected_revision: None,
            draw_order: options::look_of(&m).1,
            transparency: options::look_of(&m).0,
        }
    }

    /// Every closed object's regions, each part by itself from a point inside it.
    fn plan(&mut self, cx: &Context<'_>) {
        let input = self.request(cx);
        self.plan =
            entities_hatch::regions_in(cx.doc, Some(cx.spatial.store()), &input, &mut self.cache);
    }

    /// Writes every region's hatch in one step.
    fn write(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        self.plan(cx);
        if self.plan.is_empty() {
            cx.say(Level::Warn, "Çoklu tara: taranacak bölge kalmadı; adalar ya da yazılar alanların hepsini kaplıyor.");
            return Flow::Exit;
        }
        let (islands, texts) = self.plan.iter().fold((0, 0), |(i, t), p| match &p.tie {
            Some(tie) => (i + tie.islands.len(), t + tie.cutouts.len()),
            None => (i, t),
        });
        let input = self.request(cx);
        let result = {
            let mut ecx = ExecutionContext::with_store(cx.doc, cx.spatial.store());
            entities_hatch::execute(&mut ecx, input)
        };
        let Some(out) = points::written(result, cx) else {
            return Flow::Stay;
        };
        let name = options::choice(&self.memory).name;
        cx.say(
            Level::Success,
            format!(
                "{LABEL}: {} {name} tarama eklendi{}",
                out.created.len(),
                left_out(islands, texts)
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
        self.user = options::user_names(cx);
    }

    /// The selection's closed objects; none: said, and the tool leaves.
    fn begin(&mut self, cx: &mut Context<'_>) -> Flow {
        self.asking = None;
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
            options::with_pattern_options(prompt, &self.memory, &self.user),
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
            options::with_pattern_options(prompt, &self.memory, &self.user),
            &self.memory,
            true,
        )
    }

    /// A click answers Başlangıç's point or Özellik al's hatch (docs/adr/0331 §2).
    fn point(&mut self, p: Vec2, cx: &mut Context<'_>) -> Flow {
        if options::clicked(&mut self.asking, p, p, cx) {
            self.see(cx);
            self.plan(cx);
        }
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
        matches!(self.asking, Some(Asking::Origin | Asking::Inherit))
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
        options::choices_of(key, &self.memory, &self.user)
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
