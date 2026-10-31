//! Görünüş tarifi (docs/adr/0774 §2): the chosen named view's recipe in the Görünümler window: its level (and the one
//! under it faint), phase, layer state, scale and the categories it leaves out. A sheet's map linked to the view draws
//! with it; the model and the screen stay as they are. Hazır ▾ fills the draft from a preset (Uygulama planı 1:50, Mobilya
//! planı 1:100 …), the level kept; Tarifi uygula writes it with `cad.views.edit`'s `recipe` (its version one more),
//! Tarifi kaldır takes it away.

use iced::widget::{Column, container, row};
use iced::{Element, Fill};
use kentos_commands::{PhaseView, RecipeLevel, ViewCategory, ViewRecipe, ViewsOp};
use kentos_interaction::Level;
use kentos_ui::widget::{Choice, Select};

use crate::app::{App, Message};
use crate::exchange::words;
use crate::views::Event;

/// A ready recipe: its name and what it sets (the level is the draft's).
pub struct Preset {
    pub name: &'static str,
    pub scale: u32,
    pub phase: Option<PhaseView>,
    pub hidden: &'static [ViewCategory],
}

/// The ready recipes (docs/adr/0774 §2): a few useful ones, no more.
pub const PRESETS: [Preset; 5] = [
    Preset {
        name: "Uygulama planı 1:50",
        scale: 50,
        phase: None,
        hidden: &[],
    },
    Preset {
        name: "Mobilya planı 1:100",
        scale: 100,
        phase: None,
        hidden: &[ViewCategory::Dimensions, ViewCategory::Grids],
    },
    Preset {
        name: "Mahal planı 1:100",
        scale: 100,
        phase: None,
        hidden: &[ViewCategory::Dimensions, ViewCategory::Blocks],
    },
    Preset {
        name: "Yıkım planı 1:100",
        scale: 100,
        phase: Some(PhaseView::Demolition),
        hidden: &[ViewCategory::Blocks],
    },
    Preset {
        name: "Yeni durum 1:100",
        scale: 100,
        phase: Some(PhaseView::Proposed),
        hidden: &[],
    },
];

/// The scales the window offers; none: the view's box fitted to the map.
const SCALES: [Option<u32>; 7] = [
    None,
    Some(20),
    Some(50),
    Some(100),
    Some(200),
    Some(500),
    Some(1000),
];

const PHASES: [Option<PhaseView>; 5] = [
    None,
    Some(PhaseView::All),
    Some(PhaseView::Existing),
    Some(PhaseView::Demolition),
    Some(PhaseView::Proposed),
];

fn phase_label(p: Option<PhaseView>) -> &'static str {
    match p {
        None => "Projeninki",
        Some(PhaseView::All) => "Tümü",
        Some(PhaseView::Existing) => "Mevcut durum",
        Some(PhaseView::Demolition) => "Yıkım planı",
        Some(PhaseView::Proposed) => "Yeni durum",
    }
}

/// A recipe's words: “1:50 · Zemin kat · Yıkım planı · 2 tür gizli · sürüm 3”.
pub fn recipe_caption(app: &App, r: &ViewRecipe) -> String {
    let mut parts = Vec::new();
    if let Some(s) = r.scale {
        parts.push(format!("1:{s}"));
    }
    if let Some(l) = &r.level {
        parts.push(app.recipe_level_name(&l.id).unwrap_or_else(|| l.id.clone()));
    }
    if let Some(p) = r.phase {
        parts.push(phase_label(Some(p)).to_owned());
    }
    if let Some(s) = &r.layer_state {
        parts.push(s.clone());
    }
    if !r.hidden.is_empty() {
        parts.push(format!("{} tür gizli", r.hidden.len()));
    }
    parts.push(format!("sürüm {}", r.version));
    parts.join(" · ")
}

impl App {
    /// The project's level of `id`, by its name.
    pub(crate) fn recipe_level_name(&self, id: &str) -> Option<String> {
        self.document
            .as_ref()?
            .model
            .settings()
            .arch
            .as_ref()?
            .levels
            .iter()
            .find(|l| l.id == id)
            .map(|l| l.name.clone())
    }

    /// The recipe of the view named `name`, or an empty draft.
    pub(crate) fn recipe_draft(&self, name: &str) -> ViewRecipe {
        self.named_views()
            .iter()
            .find(|v| v.name == name)
            .and_then(|v| v.recipe.clone())
            .unwrap_or_default()
    }

    /// The draft changed by `event`; Tarifi uygula and Tarifi kaldır write.
    pub(crate) fn recipe_event(&mut self, event: Event) {
        let chosen = self.views_window.as_ref().and_then(|w| w.chosen.clone());
        let Some(name) = chosen else { return };
        match event {
            Event::RecipeApply => {
                let draft = self
                    .views_window
                    .as_ref()
                    .map(|w| w.recipe.clone())
                    .unwrap_or_default();
                if let Some(done) = self.edit_views(ViewsOp::Recipe {
                    name: name.clone(),
                    recipe: Some(draft),
                }) && let Some(r) = done.view.and_then(|v| v.recipe)
                {
                    let words = recipe_caption(self, &r);
                    self.say(
                        Level::Success,
                        format!("“{name}” görünümünün tarifi yazıldı: {words}. Bağlı paftalar bununla çizer."),
                    );
                    if let Some(w) = self.views_window.as_mut() {
                        w.recipe = r;
                    }
                }
                return;
            }
            Event::RecipeClear => {
                if self
                    .edit_views(ViewsOp::Recipe {
                        name: name.clone(),
                        recipe: None,
                    })
                    .is_some()
                {
                    self.say(
                        Level::Success,
                        format!("“{name}” görünümünün tarifi kaldırıldı."),
                    );
                    if let Some(w) = self.views_window.as_mut() {
                        w.recipe = ViewRecipe::default();
                    }
                }
                return;
            }
            _ => {}
        }
        let levels: Vec<String> = self
            .document
            .as_ref()
            .and_then(|d| d.model.settings().arch.as_ref())
            .map(|a| a.levels.iter().map(|l| l.id.clone()).collect())
            .unwrap_or_default();
        let states: Vec<String> = self
            .document
            .as_ref()
            .map(|d| {
                d.model
                    .settings()
                    .layer_states
                    .iter()
                    .map(|s| s.name.clone())
                    .collect()
            })
            .unwrap_or_default();
        let Some(w) = self.views_window.as_mut() else {
            return;
        };
        let r = &mut w.recipe;
        match event {
            Event::RecipePreset(i) => {
                if let Some(p) = PRESETS.get(i) {
                    r.scale = Some(p.scale);
                    r.phase = p.phase;
                    r.hidden = p.hidden.to_vec();
                }
            }
            Event::RecipeLevel(i) => {
                let below = r.level.as_ref().is_some_and(|l| l.below);
                r.level = i.and_then(|i| levels.get(i)).map(|id| RecipeLevel {
                    id: id.clone(),
                    below,
                });
            }
            Event::RecipeBelow(on) => {
                if let Some(l) = r.level.as_mut() {
                    l.below = on;
                }
            }
            Event::RecipePhase(i) => r.phase = PHASES.get(i).copied().flatten(),
            Event::RecipeState(i) => r.layer_state = i.and_then(|i| states.get(i)).cloned(),
            Event::RecipeScale(i) => r.scale = SCALES.get(i).copied().flatten(),
            Event::RecipeHide(c, on) => {
                r.hidden.retain(|h| *h != c);
                if on {
                    r.hidden.push(c);
                    // Kept in the categories' own order, as the recipe writes them.
                    r.hidden.sort();
                }
            }
            _ => {}
        }
    }

    /// The recipe part of the window, for the chosen view.
    pub(crate) fn recipe_view(&self, editable: bool) -> Element<'_, Message> {
        let Some(w) = &self.views_window else {
            return iced::widget::text("").into();
        };
        let m = |e: Event| Message::Views(e);
        let r = &w.recipe;
        let settings = self.document.as_ref().map(|d| d.model.settings());
        let levels: Vec<(String, String)> = settings
            .and_then(|s| s.arch.as_ref())
            .map(|a| {
                a.levels
                    .iter()
                    .map(|l| (l.id.clone(), l.name.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let states: Vec<String> = settings
            .map(|s| s.layer_states.iter().map(|s| s.name.clone()).collect())
            .unwrap_or_default();
        let preset = Select::new(
            PRESETS.iter().map(|p| Choice::new(p.name)),
            None,
            move |i| m(Event::RecipePreset(i)),
        )
        .placeholder("Hazır tarif…")
        .searchable(false);
        let level = Select::new(
            std::iter::once(Choice::new("Haritanınki".to_owned()))
                .chain(levels.iter().map(|(_, n)| Choice::new(n.clone()))),
            Some(
                r.level
                    .as_ref()
                    .and_then(|l| levels.iter().position(|(id, _)| *id == l.id))
                    .map_or(0, |i| i + 1),
            ),
            move |i| m(Event::RecipeLevel(i.checked_sub(1))),
        )
        .searchable(false);
        let below = words::check(
            r.level.as_ref().is_some_and(|l| l.below),
            "Alt kat soluk",
            r.level.as_ref().map(|l| m(Event::RecipeBelow(!l.below))),
        );
        let phase = Select::new(
            PHASES.iter().map(|p| Choice::new(phase_label(*p))),
            PHASES.iter().position(|p| *p == r.phase),
            move |i| m(Event::RecipePhase(i)),
        )
        .searchable(false);
        let state = Select::new(
            std::iter::once(Choice::new("Haritanınki".to_owned()))
                .chain(states.iter().map(|s| Choice::new(s.clone()))),
            Some(
                r.layer_state
                    .as_ref()
                    .and_then(|s| states.iter().position(|x| x == s))
                    .map_or(0, |i| i + 1),
            ),
            move |i| m(Event::RecipeState(i.checked_sub(1))),
        )
        .searchable(false);
        let scale = Select::new(
            SCALES.iter().map(|s| {
                Choice::new(s.map_or_else(|| "Görünüme sığdır".to_owned(), |s| format!("1:{s}")))
            }),
            SCALES.iter().position(|s| *s == r.scale),
            move |i| m(Event::RecipeScale(i)),
        )
        .searchable(false);
        let hide = ViewCategory::ALL
            .iter()
            .fold(iced::widget::Row::new().spacing(10), |acc, c| {
                let on = r.hidden.contains(c);
                acc.push(words::check(
                    on,
                    c.label(),
                    Some(m(Event::RecipeHide(*c, !on))),
                ))
            })
            .wrap();
        let version = if r.version > 0 {
            format!(
                "Sürüm {}. Bu görünüme bağlı pafta haritaları bu tarifle çizer; model ve ekran değişmez.",
                r.version
            )
        } else {
            "Tarif yok: bağlı haritalar kendi kat ve katman ayarlarıyla çizer.".to_owned()
        };
        let actions = row![
            words::primary(
                "Tarifi uygula",
                (editable && !r.is_empty()).then(|| m(Event::RecipeApply))
            ),
            words::secondary(
                "Tarifi kaldır",
                (editable && r.version > 0).then(|| m(Event::RecipeClear))
            ),
        ]
        .spacing(8);
        let cell =
            |title: &'static str, e: Element<'static, Message>| -> Element<'static, Message> {
                container(words::field(title, e, None)).width(Fill).into()
            };
        Column::new()
            .spacing(10)
            .push(words::field("Görünüş tarifi", preset, Some(version)))
            .push(row![cell("Kat", level.into()), cell("Evre", phase.into())].spacing(12))
            .push(below)
            .push(
                row![
                    cell("Katman durumu", state.into()),
                    cell("Ölçek", scale.into())
                ]
                .spacing(12),
            )
            .push(words::field("Gizlenen türler", hide, None))
            .push(actions)
            .into()
    }
}
