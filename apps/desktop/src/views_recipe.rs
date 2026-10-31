//! Görünüş tarifi (docs/adr/0774 §2): the chosen named view's recipe in the Görünümler window: its level (and the one
//! under it faint), phase, layer state, scale, the categories it leaves out, the walls' detail and the crop to the
//! view's area (docs/adr/0781), the template it follows, black and white and the categories' pen weights (docs/adr/0783).
//! A sheet's map linked to the view draws
//! with it; the model and the screen stay as they are. Hazır ▾ fills the draft from a preset (Uygulama planı 1:50, Mobilya
//! planı 1:100 …), the level kept; Tarifi uygula writes it with `cad.views.edit`'s `recipe` (its version one more),
//! Tarifi kaldır takes it away.

use iced::widget::{Column, container, row};
use iced::{Element, Fill};
use kentos_commands::{
    PenCategory, PhaseView, RecipeLevel, RecipePen, ViewCategory, ViewRecipe, ViewTemplate, ViewsOp, WallDetail,
};
use kentos_interaction::Level;
use kentos_ui::label;
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
    pub detail: Option<WallDetail>,
}

/// The ready recipes (docs/adr/0774 §2): a few useful ones, no more.
pub const PRESETS: [Preset; 5] = [
    Preset {
        name: "Uygulama planı 1:50",
        scale: 50,
        phase: None,
        hidden: &[],
        detail: Some(WallDetail::Detailed),
    },
    Preset {
        name: "Mobilya planı 1:100",
        scale: 100,
        phase: None,
        hidden: &[ViewCategory::Dimensions, ViewCategory::Grids],
        detail: None,
    },
    Preset {
        name: "Mahal planı 1:100",
        scale: 100,
        phase: None,
        hidden: &[ViewCategory::Dimensions, ViewCategory::Blocks],
        detail: None,
    },
    Preset {
        name: "Yıkım planı 1:100",
        scale: 100,
        phase: Some(PhaseView::Demolition),
        hidden: &[ViewCategory::Blocks],
        detail: None,
    },
    Preset {
        name: "Yeni durum 1:100",
        scale: 100,
        phase: Some(PhaseView::Proposed),
        hidden: &[],
        detail: None,
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

/// The walls' details the window offers (docs/adr/0781 §1); none: the scale's.
pub(crate) const DETAILS: [Option<WallDetail>; 4] = [
    None,
    Some(WallDetail::Schematic),
    Some(WallDetail::Standard),
    Some(WallDetail::Detailed),
];

/// The pen weights the window offers, paper millimetres (ISO 128's series); none: the objects' own.
pub(crate) const PEN_WEIGHTS: [f64; 7] = [0.13, 0.18, 0.25, 0.35, 0.5, 0.7, 1.0];

fn weight_label(w: f64) -> String {
    format!("{w} mm")
}

fn detail_label(d: Option<WallDetail>) -> &'static str {
    match d {
        None => "Ölçeğe göre",
        Some(WallDetail::Schematic) => "Şematik",
        Some(WallDetail::Standard) => "Uygulama",
        Some(WallDetail::Detailed) => "Ayrıntılı",
    }
}

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
    if let Some(d) = r.detail {
        parts.push(format!("duvar {}", detail_label(Some(d)).to_lowercase()));
    }
    if r.crop {
        parts.push("kırpılmış".to_owned());
    }
    if let Some(t) = &r.template {
        let name = app.view_templates().iter().find(|x| &x.id == t).map_or(t.clone(), |x| x.name.clone());
        parts.push(format!("şablon {name}"));
    }
    if !r.pens.is_empty() {
        parts.push(format!("{} kalem", r.pens.len()));
    }
    if r.mono {
        parts.push("siyah beyaz".to_owned());
    }
    // The design options it shows (docs/adr/0788 §3), by their names.
    let sets = app.document.as_ref().map_or(&[][..], |d| d.model.arch_options());
    for c in &r.options {
        let words = sets
            .iter()
            .find(|s| s.id == c.set)
            .and_then(|s| s.options.iter().find(|o| o.id == c.option).map(|o| format!("{} {}", s.name, o.name)))
            .unwrap_or_else(|| format!("{} {}", c.set, c.option));
        parts.push(words);
    }
    parts.push(format!("sürüm {}", r.version));
    parts.join(" · ")
}

impl App {
    /// The project's view templates (docs/adr/0783 §2).
    pub(crate) fn view_templates(&self) -> &[ViewTemplate] {
        self.document
            .as_ref()
            .map_or(&[][..], |d| d.model.settings().view_templates.as_slice())
    }

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
            Event::TemplateSave => {
                let Some(w) = self.views_window.as_ref() else { return };
                let tname = w.template_name.trim().to_owned();
                if tname.is_empty() {
                    self.warn("Şablona bir ad verin.");
                    return;
                }
                // A template is what the view draws, its level and crop aside (docs/adr/0783 §2).
                let r = kentos_commands::resolved_recipe(&w.recipe, self.view_templates());
                let recipe = ViewRecipe {
                    version: 1,
                    level: None,
                    crop: false,
                    template: None,
                    ..r
                };
                if recipe.is_empty() {
                    self.warn("Tarif boş: şablon olarak kaydedilecek bir şey yok.");
                    return;
                }
                if self
                    .edit_views(ViewsOp::Template {
                        name: tname.clone(),
                        recipe: Some(recipe),
                    })
                    .is_some()
                {
                    self.say(
                        Level::Success,
                        format!("“{tname}” görünüş şablonu kaydedildi; görünümler Şablon'dan ona bağlanabilir."),
                    );
                    if let Some(w) = self.views_window.as_mut() {
                        w.template_name.clear();
                    }
                }
                return;
            }
            _ => {}
        }
        let templates: Vec<ViewTemplate> = self.view_templates().to_vec();
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
        let sets: Vec<kentos_commands::OptionSet> = self
            .document
            .as_ref()
            .map(|d| d.model.arch_options().to_vec())
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
                    r.detail = p.detail;
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
            Event::RecipeDetail(i) => r.detail = DETAILS.get(i).copied().flatten(),
            Event::RecipeCrop(on) => r.crop = on,
            Event::RecipeTemplate(i) => {
                let (level, options) = (r.level.clone(), r.options.clone());
                let (version, crop) = (r.version, r.crop);
                match i.and_then(|i| templates.get(i)) {
                    // Following a template: its fields come from it, the level and the crop stay the view's.
                    Some(t) => {
                        *r = ViewRecipe {
                            version,
                            level,
                            crop,
                            options,
                            template: Some(t.id.clone()),
                            ..ViewRecipe::default()
                        }
                    }
                    // Leaving it: the template's fields stay as the view's own.
                    None => {
                        if let Some(t) = r.template.as_ref().and_then(|id| templates.iter().find(|t| &t.id == id)) {
                            *r = ViewRecipe {
                                version,
                                level,
                                crop,
                                options,
                                template: None,
                                ..t.recipe.clone()
                            };
                        } else {
                            r.template = None;
                        }
                    }
                }
            }
            Event::RecipeMono(on) => r.mono = on,
            Event::RecipePen(c, i) => {
                let color = r.pen(c).and_then(|p| p.color.clone());
                r.pens.retain(|p| p.of != c);
                let weight = i.and_then(|i| PEN_WEIGHTS.get(i)).copied();
                if weight.is_some() || color.is_some() {
                    r.pens.push(RecipePen { of: c, weight, color });
                    r.pens.sort_by_key(|p| p.of);
                }
            }
            Event::TemplateName(name) => w.template_name = name,
            Event::RecipeOption(i, option) => {
                if let Some(set) = sets.get(i) {
                    r.options.retain(|c| c.set != set.id);
                    if let Some(o) = option.and_then(|j| set.options.get(j)) {
                        r.options.push(kentos_commands::OptionChoice {
                            set: set.id.clone(),
                            option: o.id.clone(),
                        });
                        // Kept in the sets' own order.
                        r.options.sort_by_key(|c| sets.iter().position(|s| s.id == c.set));
                    }
                }
            }
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
        let detail = Select::new(
            DETAILS.iter().map(|d| Choice::new(detail_label(*d))),
            DETAILS.iter().position(|d| *d == r.detail),
            move |i| m(Event::RecipeDetail(i)),
        )
        .searchable(false);
        let crop = words::check(
            r.crop,
            "Bağlı haritaları görünümün alanına kırp",
            Some(m(Event::RecipeCrop(!r.crop))),
        );
        // A design option set's choice each (docs/adr/0788 §3): its primary unless another is chosen.
        let sets = self.document.as_ref().map_or(&[][..], |d| d.model.arch_options());
        let mut options = Column::new().spacing(6);
        for (i, s) in sets.iter().enumerate() {
            let primary = s.options.iter().find(|o| o.id == s.primary).map_or("", |o| o.name.as_str());
            let chosen = r
                .options
                .iter()
                .find(|c| c.set == s.id)
                .and_then(|c| s.options.iter().position(|o| o.id == c.option))
                .map_or(0, |j| j + 1);
            let select = Select::new(
                std::iter::once(Choice::new(format!("Birincil ({primary})")))
                    .chain(s.options.iter().map(|o| Choice::new(o.name.clone()))),
                Some(chosen),
                move |j| m(Event::RecipeOption(i, j.checked_sub(1))),
            )
            .searchable(false);
            options = options.push(words::field(
                if i == 0 { "Tasarım seçeneği" } else { "" },
                row![container(label::caption(s.name.clone())).width(120.0), container(select).width(Fill)]
                    .spacing(8)
                    .align_y(iced::Center),
                None,
            ));
        }
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
            .wrap().vertical_spacing(6);
        let templates = self.view_templates();
        let template = Select::new(
            std::iter::once(Choice::new("Yok".to_owned()))
                .chain(templates.iter().map(|t| Choice::new(t.name.clone()))),
            Some(
                r.template
                    .as_ref()
                    .and_then(|id| templates.iter().position(|t| &t.id == id))
                    .map_or(0, |i| i + 1),
            ),
            move |i| m(Event::RecipeTemplate(i.checked_sub(1))),
        )
        .searchable(false);
        let followed = r
            .template
            .as_ref()
            .and_then(|id| templates.iter().find(|t| &t.id == id));
        let mono = words::check(
            r.mono,
            "Siyah beyaz: çizgiler siyah, taramalar gri",
            Some(m(Event::RecipeMono(!r.mono))),
        );
        let pens = PenCategory::ALL
            .iter()
            .fold(iced::widget::Row::new().spacing(12), |acc, c| {
                let c = *c;
                let at = r
                    .pen(c)
                    .and_then(|p| p.weight)
                    .and_then(|w| PEN_WEIGHTS.iter().position(|x| (*x - w).abs() < 1e-9));
                let pick = Select::new(
                    std::iter::once(Choice::new("Kendi".to_owned()))
                        .chain(PEN_WEIGHTS.iter().map(|w| Choice::new(weight_label(*w)))),
                    Some(at.map_or(0, |i| i + 1)),
                    move |i| m(Event::RecipePen(c, i.checked_sub(1))),
                )
                .searchable(false);
                acc.push(
                    container(words::field(c.label(), pick, None)).width(150),
                )
            })
            .wrap().vertical_spacing(6);
        let save = row![
            container(kentos_ui::widget::text_field("Şablonun adı", &w.template_name)
                .on_input(move |t| m(Event::TemplateName(t)))
                .on_submit(m(Event::TemplateSave)))
            .width(Fill),
            words::secondary(
                "Şablon olarak kaydet",
                (editable && !r.is_empty() && !w.template_name.trim().is_empty())
                    .then(|| m(Event::TemplateSave))
            ),
        ]
        .spacing(8)
        .align_y(iced::Center);
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
        let mut body = Column::new()
            .spacing(10)
            .push(words::field("Görünüş tarifi", preset, Some(version)))
            .push(row![cell("Şablon", template.into()), cell("Kat", level.into())].spacing(12))
            .push(below)
            .push(options);
        match followed {
            // Following a template: the level and the crop are the view's, the rest the template's (docs/adr/0783 §2).
            Some(t) => {
                body = body
                    .push(container(crop).width(Fill))
                    .push(words::field(
                        "Şablondan",
                        kentos_ui::label::muted(recipe_caption(self, &t.recipe)),
                        Some("Evre, katman durumu, ölçek, ayrıntı, gizlenenler ve kalemler şablondan gelir; şablon değişince bağlı paftalar da değişir.".to_owned()),
                    ));
            }
            None => {
                body = body
                    .push(
                        row![
                            cell("Evre", phase.into()),
                            cell("Katman durumu", state.into()),
                            cell("Ölçek", scale.into())
                        ]
                        .spacing(12),
                    )
                    .push(
                        row![
                            cell("Duvar ayrıntısı", detail.into()),
                            container(crop).width(Fill).padding([22, 0])
                        ]
                        .spacing(12),
                    )
                    .push(words::field("Gizlenen türler", hide, None))
                    .push(mono)
                    .push(words::field("Kalem kalınlıkları (kâğıtta)", pens, None));
            }
        }
        body.push(actions).push(save).into()
    }
}
