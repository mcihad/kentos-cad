//! Planı tanı (plan/architecture-advanced.md M48, docs/adr/8048 §4): an office's exploded plan — its walls, doors and
//! windows come in as lines and arcs — read back as walls and their openings, selection first as the modify tools
//! (`modify`).
//!
//! - Each layer of the selection is a row of the tool's settings with its role (Duvar, Kapı, Pencere, Kolon or Yok
//!   say; from its name, `arch_recognize::layer_roles`), changed there;
//! - Kat (L) as every Mimari tool's; Kaynak çizgiler (K) as Çizgiden duvar's: Sil takes the lines read away;
//! - the walls and the openings to come are shown from the command's plan, what is read and what stays beside the
//!   cursor; Enter, Uygula or a quick right click writes them through `cad.arch.recognize` in one undo step
//!   “Planı tanı”.

use std::collections::BTreeMap;

use kentos_commands::{ArchRecognize, CommandResult, Entity, OpeningForm, PlanLayerRole, PlanRole};
use kentos_geometry_core::geom::affine::Affine;
use kentos_geometry_core::jsmath::{cos, sin};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::{ExecutionContext, arch_recognize};

use crate::Vec2;
use crate::architecture_level as level;
use crate::format::Format;
use crate::log::Level;
use crate::modify::{Modify, Stages};
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Memory, OptionChoice, Pointer, Preview, Stroke, Tag, Tone};
use crate::tool_settings::{Setting, choices};
use crate::wall::{self, Drawn};

pub const ID: &str = "planRecognize";
pub const LABEL: &str = "Planı tanı";

/// The settings' keys of the selection's layers, one row each; a layer past them keeps its name's role.
const ROLE_KEYS: [&str; 24] = [
    "R1", "R2", "R3", "R4", "R5", "R6", "R7", "R8", "R9", "R10", "R11", "R12", "R13", "R14", "R15",
    "R16", "R17", "R18", "R19", "R20", "R21", "R22", "R23", "R24",
];

/// A role as the settings offer it.
const ROLES: [(PlanRole, &str, &str); 5] = [
    (PlanRole::Wall, "wall", "Duvar"),
    (PlanRole::Door, "door", "Kapı"),
    (PlanRole::Window, "window", "Pencere"),
    (PlanRole::Column, "column", "Kolon"),
    (PlanRole::Ignore, "ignore", "Yok say"),
];

fn role_word(role: PlanRole) -> &'static str {
    ROLES.iter().find(|r| r.0 == role).map_or("", |r| r.1)
}

/// What was planned, and for what: the drawing's generation, the sources kept, the storey and the roles given.
#[derive(Clone, Debug, Default)]
struct Plan {
    key: (u64, bool, Option<String>, Vec<(String, PlanRole)>),
    drawn: Drawn,
    walls: usize,
    doors: usize,
    windows: usize,
    bare: usize,
    columns: usize,
    removed: usize,
    left: usize,
    refused: Option<String>,
}

#[derive(Default)]
pub struct PlanRecognize {
    uids: Vec<String>,
    /// The selection's layers: their ids and names, by name.
    layers: Vec<(String, String)>,
    /// The roles the user chose here, over their names'.
    given: BTreeMap<String, PlanRole>,
    /// Each layer's role as the command reads it.
    roles: BTreeMap<String, PlanRole>,
    hover: Option<Vec2>,
    plan: Plan,
    seen: Memory,
    level: level::Choice,
}

impl PlanRecognize {
    pub fn tool() -> Modify<Self> {
        Modify::with(Self::default())
    }

    fn given(&self) -> Vec<PlanLayerRole> {
        self.given
            .iter()
            .map(|(id, role)| PlanLayerRole {
                layer_id: id.clone(),
                role: *role,
            })
            .collect()
    }

    fn request(&self, cx: &Context<'_>) -> ArchRecognize {
        ArchRecognize {
            uids: self.uids.clone(),
            roles: self.given(),
            arch: cx.phased(self.level.arch()),
            wall_layer_id: None,
            opening_layer_id: None,
            min_width: None,
            max_width: None,
            max_opening: None,
            angle: None,
            keep_sources: cx.memory.wall_keep_sources,
            expected_revision: None,
        }
    }

    fn replan(&mut self, cx: &Context<'_>) {
        self.roles = arch_recognize::layer_roles(
            cx.doc,
            self.layers.iter().map(|(id, _)| id.as_str()),
            &self.given(),
        );
        let key = (
            cx.doc.generation(),
            cx.memory.wall_keep_sources,
            self.level.id(),
            self.given.iter().map(|(k, v)| (k.clone(), *v)).collect(),
        );
        if self.uids.is_empty() || self.plan.key == key {
            return;
        }
        let mut plan = Plan {
            key,
            ..Plan::default()
        };
        match arch_recognize::plan_in(cx.doc, &self.request(cx)) {
            CommandResult::Completed { output, .. } => {
                plan.walls = output.walls.len();
                plan.columns = output.columns.len();
                plan.removed = output.removed.len();
                plan.left = output.left.len();
                for e in output.walls.iter().take(crate::modify::MAX_GHOSTS) {
                    wall::add(e, &mut plan.drawn);
                }
                for e in &output.openings {
                    let Entity::Opening(o) = e else { continue };
                    match o.opening.form {
                        OpeningForm::Door => plan.doors += 1,
                        OpeningForm::Window => plan.windows += 1,
                        _ => plan.bare += 1,
                    }
                    // Its jambs across the wall: start's left and right, end's left and right.
                    if let [a, b, c, d] = o.opening.place[..] {
                        let pts = [a, c, d, b]
                            .into_iter()
                            .map(|p| Vec2::new(p.x, p.y))
                            .collect();
                        plan.drawn
                            .strokes
                            .push(Stroke::solid(pts, true).width(2.0).tone(Tone::Snap));
                    }
                }
                // Each column's outline: a rectangle turned as it is, a round one in 24 sides.
                for e in &output.columns {
                    let Entity::Column(c) = e else { continue };
                    let c = &c.column;
                    let pts: Vec<Vec2> = if c.shape == kentos_commands::ColumnShape::Round {
                        (0..24)
                            .map(|k| {
                                let t = f64::from(k) * std::f64::consts::PI / 12.0;
                                Vec2::new(
                                    c.at.x + c.width / 2.0 * cos(t),
                                    c.at.y + c.width / 2.0 * sin(t),
                                )
                            })
                            .collect()
                    } else {
                        let (w, d) = (c.width / 2.0, c.depth.unwrap_or(c.width) / 2.0);
                        let (cs, sn) = (cos(c.rotation), sin(c.rotation));
                        [(-w, -d), (w, -d), (w, d), (-w, d)]
                            .into_iter()
                            .map(|(a, b)| {
                                Vec2::new(c.at.x + a * cs - b * sn, c.at.y + a * sn + b * cs)
                            })
                            .collect()
                    };
                    plan.drawn
                        .strokes
                        .push(Stroke::solid(pts, true).width(2.0).tone(Tone::Snap));
                }
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => plan.refused = Some(error.message),
            CommandResult::Queued { .. } | CommandResult::Cancelled => {}
        }
        self.plan = plan;
    }

    fn finding(&self) -> String {
        let p = &self.plan;
        if let Some(why) = &p.refused {
            return why.clone();
        }
        let mut out = format!(
            "{} duvar, {} kapı, {} pencere, {} boşluk",
            p.walls, p.doors, p.windows, p.bare
        );
        if p.columns > 0 {
            out.push_str(&format!(", {} kolon", p.columns));
        }
        if p.removed > 0 {
            out.push_str(&format!("; {} nesne silinir", p.removed));
        }
        if p.left > 0 {
            out.push_str(&format!("; {} nesne kalır", p.left));
        }
        out
    }

    /// The row of the layer at `k`: its name and its role's choices.
    fn layer_row(&self, k: usize) -> Option<Setting> {
        let (id, name) = self.layers.get(k)?;
        let role = self.roles.get(id).copied().unwrap_or(PlanRole::Ignore);
        let items: Vec<(&str, &str)> = ROLES.iter().map(|r| (r.1, r.2)).collect();
        Some(Setting::choice(
            ROLE_KEYS.get(k)?,
            name.clone(),
            choices(&items, role_word(role)),
        ))
    }
}

impl Stages for PlanRecognize {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = *cx.memory;
        self.level.see(cx.doc.settings());
        self.level.follow(cx.level);
        self.replan(cx);
    }

    fn begin(&mut self, cx: &mut Context<'_>) -> Flow {
        self.plan = Plan::default();
        self.given.clear();
        let slots = cx.selection.ids();
        self.uids = slots
            .iter()
            .filter_map(|&s| cx.doc.uid(s).map(|u| u.to_string()))
            .collect();
        if self.uids.is_empty() {
            cx.say(Level::Warn, "Planı tanı: önce planın nesnelerini seçin.");
            return Flow::Exit;
        }
        let tree = cx.doc.layers();
        let mut layers: Vec<(String, String)> = Vec::new();
        for e in slots.iter().filter_map(|&s| cx.doc.get(s)) {
            let id = &e.base().layer_id;
            if !layers.iter().any(|(l, _)| l == id) {
                let name = tree.get(id).map_or_else(|| id.clone(), |n| n.name.clone());
                layers.push((id.clone(), name));
            }
        }
        layers.sort_by_key(|(_, name)| upper_tr(name));
        self.layers = layers;
        self.see(cx);
        Flow::Stay
    }

    fn picking_hint(&self, prompt: Prompt) -> Prompt {
        prompt.note("tanınacak planın nesnelerini seçin (pencereyle bütün kat)")
    }

    fn snaps(&self) -> bool {
        false
    }

    fn anchor(&self) -> Option<Vec2> {
        None
    }

    fn prompt(&self, _n: usize) -> Prompt {
        Prompt::new(LABEL, self.finding())
            .option_with("Kat", "L", self.level.label())
            .option_with(
                "Kaynak çizgiler",
                "K",
                if self.seen.wall_keep_sources {
                    "Koru"
                } else {
                    "Sil"
                },
            )
            .option("Uygula", "Enter")
    }

    fn pointer(&mut self, p: &Pointer, down: bool, _cx: &mut Context<'_>) -> Option<Flow> {
        if down {
            return Some(Flow::Stay);
        }
        self.hover = Some(p.raw);
        None
    }

    fn point(&mut self, _p: Vec2, _cx: &mut Context<'_>) -> Flow {
        Flow::Stay
    }

    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> Option<Flow> {
        match upper_tr(js_trim(text)).as_str() {
            "K" => cx.memory.wall_keep_sources = !cx.memory.wall_keep_sources,
            "L" => self.level.next(),
            _ => return None,
        }
        self.see(cx);
        Some(Flow::Stay)
    }

    fn typed_points(&self) -> bool {
        false
    }

    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        self.settings(&Format::default())
            .into_iter()
            .find(|s| s.key == key)
            .map_or_else(Vec::new, |s| s.choices)
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        self.configure(key, typed, cx)
    }

    fn settings(&self, _f: &Format) -> Vec<Setting> {
        let mut rows = vec![
            self.level.setting(),
            Setting::choice(
                "K",
                "Kaynak çizgiler",
                choices(
                    &[("keep", "Koru"), ("remove", "Sil")],
                    if self.seen.wall_keep_sources {
                        "keep"
                    } else {
                        "remove"
                    },
                ),
            ),
            Setting::hint(
                "N",
                "Katmanlar",
                "Her katmanın çizgileri rolüyle okunur; rolü adından gelir, buradan değiştirilir.",
            ),
        ];
        rows.extend((0..self.layers.len()).filter_map(|k| self.layer_row(k)));
        if self.layers.len() > ROLE_KEYS.len() {
            rows.push(Setting::hint(
                "M",
                "Diğerleri",
                format!(
                    "{} katman daha: rolleri adlarından.",
                    self.layers.len() - ROLE_KEYS.len()
                ),
            ));
        }
        rows
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        let value = value.trim();
        let taken = if key == "L" {
            self.level.choose(value)
        } else if key == "K" {
            match value {
                "keep" | "Koru" => {
                    cx.memory.wall_keep_sources = true;
                    true
                }
                "remove" | "Sil" => {
                    cx.memory.wall_keep_sources = false;
                    true
                }
                _ => false,
            }
        } else if let Some(k) = ROLE_KEYS.iter().position(|r| *r == key)
            && let Some((id, _)) = self.layers.get(k)
            && let Some(role) = ROLES
                .iter()
                .find(|r| r.1 == value || upper_tr(r.2) == upper_tr(value))
        {
            self.given.insert(id.clone(), role.0);
            true
        } else {
            false
        };
        if !taken {
            cx.say(
                Level::Warn,
                "Değer uygulanmadı; bir kat, Koru ya da Sil, ya da Duvar, Kapı, Pencere, Kolon, Yok say seçin.",
            );
        }
        self.see(cx);
        taken
    }

    /// Enter: the walls and openings written in one step, and the tool leaves; a refusal is said and it stays.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        let input = self.request(cx);
        let result = arch_recognize::execute(&mut ExecutionContext::new(cx.doc), input);
        match points::written(result, cx) {
            Some(done) => {
                let mut said = format!(
                    "{LABEL}: {} duvar, {} kapı, {} pencere, {} boşluk",
                    done.walls.len(),
                    done.doors,
                    done.windows,
                    done.bare
                );
                if !done.columns.is_empty() {
                    said.push_str(&format!(", {} kolon", done.columns.len()));
                }
                said.push_str(" eklendi");
                if !done.removed.is_empty() {
                    said.push_str(&format!(", {} nesne silindi", done.removed.len()));
                }
                if !done.left.is_empty() {
                    said.push_str(&format!(", {} nesne çizimde kaldı", done.left.len()));
                }
                cx.say(Level::Success, format!("{said}."));
                Flow::Exit
            }
            None => Flow::Stay,
        }
    }

    fn preview(&self, _hover: Vec2) -> Option<Affine> {
        None
    }

    fn stage_preview(&self, hover: Option<Vec2>, _format: &Format) -> Option<Preview> {
        let tag = hover.map(|at| Tag {
            at,
            lines: vec![self.finding(), "Enter: uygula".to_owned()],
        });
        Some(Preview {
            areas: self.plan.drawn.areas.clone(),
            strokes: self.plan.drawn.strokes.clone(),
            tag,
            tag_tone: if self.plan.refused.is_some() {
                Tone::Danger
            } else {
                Tone::Accent
            },
            ..Preview::default()
        })
    }
}
