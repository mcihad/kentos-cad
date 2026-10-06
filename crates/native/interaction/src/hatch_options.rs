//! Tarama's and Çoklu tara's pattern options (docs/adr/0186 §4): Desen (D)
//! with its menu, Ölçek (Ö) and Açı (Ç), İkinci renk (R) and Ters (T) for a
//! gradient, Adalar (A), İlişkili (İ) and Yazılar (Y); the values asked in
//! the command line, and the pattern a hatch is written with. The choices
//! and the pattern are the core's (`tools::hatch`); the web's are
//! `tools/hatchOptions.ts`.

use kentos_contracts::HatchPattern;
use kentos_geometry_core::tools::hatch::{
    COLOURS, Choice, Kind, Options, choice_named, choices, colour_of, pattern_of,
};
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::geometry::contract_pattern;

use crate::format::js_number;
use crate::log::Level;
use crate::prompt::Prompt;
use crate::tool::{Context, Memory, OptionChoice};

/// What an option asks for while it is asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asking {
    Scale,
    Angle,
    Colour,
}

/// The largest Ölçek taken.
const MOST_SCALE: f64 = 10_000.0;

/// The session's choice.
pub fn choice(m: &Memory) -> Choice {
    let list = choices();
    let k = m.hatch_choice.min(list.len().saturating_sub(1));
    list.into_iter().nth(k).unwrap_or_else(|| Choice {
        name: "Dolu".into(),
        label: "Dolu".into(),
        typed: "dolu".into(),
        icon: "hatchSwatchSolid".into(),
        preview: "hatchPreviewSolid".into(),
        kind: Kind::Solid,
        class: "solid",
    })
}

/// İkinci renk as written, `#RRGGBB`.
pub fn colour_text(m: &Memory) -> String {
    format!("#{:06X}", m.hatch_color2 & 0xFF_FF_FF)
}

/// The pattern a hatch is written with at the drawing's plot scale (1:N).
pub fn pattern(m: &Memory, plot_scale: f64) -> Option<HatchPattern> {
    contract_pattern(pattern_of(&Options {
        choice: m.hatch_choice,
        scale: m.hatch_scale,
        angle: m.hatch_angle,
        color2: colour_text(m),
        inverted: m.hatch_inverted,
        plot_scale,
    }))
}

fn on(b: bool) -> &'static str {
    if b { "açık" } else { "kapalı" }
}

/// Desen and what it reads: İkinci renk and Ters for a gradient, Ölçek for
/// lines and patterns, Açı for all but Dolu.
pub fn with_pattern_options(prompt: Prompt, m: &Memory) -> Prompt {
    let c = choice(m);
    let gradient = matches!(c.kind, Kind::Gradient(_));
    let mut prompt = prompt.option_with("Desen", "D", c.name);
    if gradient {
        prompt = prompt
            .option_with("İkinci renk", "R", colour_text(m))
            .option_with("Ters", "T", on(m.hatch_inverted));
    } else if c.kind != Kind::Solid {
        prompt = prompt.option_with("Ölçek", "Ö", js_number(m.hatch_scale));
    }
    if c.kind != Kind::Solid {
        prompt = prompt.option_with("Açı", "Ç", format!("{}°", js_number(m.hatch_angle)));
    }
    prompt
}

/// Adalar, İlişkili (where the hatch can follow its objects) and Yazılar.
pub fn with_region_options(prompt: Prompt, m: &Memory, tie: bool) -> Prompt {
    prompt
        .option_with(
            "Adalar",
            "A",
            if m.hatch_islands {
                "taranmaz"
            } else {
                "taranır"
            },
        )
        .option_if(tie, "İlişkili", "İ", on(m.hatch_assoc))
        .option_with(
            "Yazılar",
            "Y",
            if m.hatch_texts {
                "boş bırakılır"
            } else {
                "taranır"
            },
        )
}

/// What a stage that asks says.
pub fn asking_prompt(label: &'static str, asking: Asking) -> Prompt {
    match asking {
        Asking::Scale => Prompt::new(
            label,
            "desenin kâğıttaki ölçeğini yazın (1: kitaplığın büyüklüğü)",
        ),
        Asking::Angle => Prompt::new(label, "desenin açısını derece olarak yazın"),
        Asking::Colour => Prompt::new(
            label,
            "ikinci rengi yazın (#RRGGBB ya da renk adı) ya da menüden seçin",
        )
        .option("İkinci renk", "R"),
    }
}

/// The options' keys: whether `key` was one of them (a key that asks for a
/// value starts asking it). `tie`: İlişkili is offered.
pub fn option(key: &str, tie: bool, asking: &mut Option<Asking>, cx: &mut Context<'_>) -> bool {
    let kind = choice(cx.memory).kind;
    let gradient = matches!(kind, Kind::Gradient(_));
    let m = &mut *cx.memory;
    match key {
        "D" => m.hatch_choice = (m.hatch_choice + 1) % choices().len(),
        "Ö" if !gradient && kind != Kind::Solid => *asking = Some(Asking::Scale),
        "Ç" if kind != Kind::Solid => *asking = Some(Asking::Angle),
        "R" if gradient => *asking = Some(Asking::Colour),
        "T" if gradient => m.hatch_inverted = !m.hatch_inverted,
        "A" => m.hatch_islands = !m.hatch_islands,
        "İ" if tie => m.hatch_assoc = !m.hatch_assoc,
        "Y" => m.hatch_texts = !m.hatch_texts,
        _ => return false,
    }
    true
}

/// A value typed for what is asked: whether it was taken (a wrong one is
/// said and asked again).
pub fn answer(asking: &mut Option<Asking>, text: &str, cx: &mut Context<'_>) -> bool {
    let Some(what) = *asking else {
        return false;
    };
    let t = js_trim(text);
    match what {
        Asking::Scale => match parse_number(t) {
            Some(n) if n > 0.0 && n <= MOST_SCALE => {
                cx.memory.hatch_scale = n;
                *asking = None;
            }
            _ => cx.say(
                Level::Warn,
                format!("Ölçek sıfırdan büyük, en çok {} bir sayı olmalı; “{t}” yazıldı.", js_number(MOST_SCALE)),
            ),
        },
        Asking::Angle => match parse_number(t) {
            Some(n) if n.is_finite() => {
                cx.memory.hatch_angle = n;
                *asking = None;
            }
            _ => cx.say(Level::Warn, format!("Açı derece olarak bir sayı olmalı; “{t}” yazıldı.")),
        },
        Asking::Colour => match colour_of(t) {
            Some(c) => {
                cx.memory.hatch_color2 = u32::from_str_radix(&c[1..], 16).unwrap_or(0xFF_FF_FF);
                *asking = None;
            }
            None => cx.say(
                Level::Warn,
                format!("İkinci renk #RRGGBB biçiminde ya da bir renk adı olmalı (beyaz, siyah, kırmızı …); “{t}” yazıldı."),
            ),
        },
    }
    true
}

/// A pattern's name typed while nothing is asked (“ansi31”, “dolu”): whether it was one.
pub fn typed_choice(text: &str, cx: &mut Context<'_>) -> bool {
    match choice_named(text) {
        Some(k) => {
            cx.memory.hatch_choice = k;
            true
        }
        None => false,
    }
}

/// Desen's and İkinci renk's menus.
pub fn choices_of(key: &str, m: &Memory) -> Vec<OptionChoice> {
    match key {
        "D" => choices()
            .into_iter()
            .enumerate()
            .map(|(k, c)| OptionChoice {
                label: c.label,
                typed: c.typed,
                icon: Some(icon_name(&c.icon)),
                preview: Some(icon_name(&c.preview)),
                checked: k == m.hatch_choice,
                command: None,
            })
            .collect(),
        "R" => {
            let now = colour_text(m);
            COLOURS
                .iter()
                .map(|(name, hex)| OptionChoice {
                    label: format!("{name} ({hex})"),
                    typed: name.to_lowercase(),
                    icon: None,
                    preview: None,
                    checked: *hex == now,
                    command: None,
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

/// One of the menus' entries chosen: whether it was taken.
pub fn choose(key: &str, typed: &str, asking: &mut Option<Asking>, cx: &mut Context<'_>) -> bool {
    match key {
        "D" => typed_choice(typed, cx),
        "R" => match colour_of(typed) {
            Some(c) => {
                cx.memory.hatch_color2 = u32::from_str_radix(&c[1..], 16).unwrap_or(0xFF_FF_FF);
                *asking = None;
                true
            }
            None => false,
        },
        _ => false,
    }
}

/// A hatch's pattern given another of Desen's choices in Öznitelikler
/// (docs/adr/0186 §7): its own Ölçek and Açı carried over as the tool's
/// (a pattern's scale or user lines' spacing on the paper, user lines' turn
/// past 45°), its gradient's colour and way.
pub fn rechosen(p: &HatchPattern, choice: usize, plot_scale: f64) -> Option<HatchPattern> {
    use kentos_contracts::HatchPatternType as T;
    let metres = plot_scale / 1000.0;
    let scale = match p.kind {
        T::Pattern => p.scale.unwrap_or(metres) / metres,
        T::Lines | T::Cross => {
            p.spacing / (kentos_geometry_core::tools::hatch::USER_SPACING_MM * metres)
        }
        _ => 1.0,
    };
    let angle = match p.kind {
        T::Lines | T::Cross => p.angle - kentos_geometry_core::tools::hatch::USER_ANGLE,
        T::Solid => 0.0,
        _ => p.angle,
    };
    let g = p.gradient.as_ref();
    contract_pattern(pattern_of(&Options {
        choice,
        scale: if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        },
        angle,
        color2: g.map_or_else(|| "#FFFFFF".to_owned(), |g| g.color2.clone()),
        inverted: g.is_some_and(|g| g.inverted),
        plot_scale,
    }))
}

/// The choice a hatch's pattern is: its library name, else its kind's.
pub fn choice_of(p: &HatchPattern) -> Option<usize> {
    use kentos_contracts::HatchPatternType as T;
    let list = choices();
    match p.kind {
        T::Pattern => p.name.as_deref().and_then(choice_named),
        T::Gradient => {
            let shape = p.gradient.as_ref().map(|g| g.shape);
            list.iter().position(|c| match (c.kind, shape) {
                (Kind::Gradient(s), Some(kentos_contracts::GradientShape::Linear)) => s == "linear",
                (Kind::Gradient(s), Some(kentos_contracts::GradientShape::Cylinder)) => {
                    s == "cylinder"
                }
                (Kind::Gradient(s), Some(kentos_contracts::GradientShape::Spherical)) => {
                    s == "spherical"
                }
                _ => false,
            })
        }
        T::Lines => Some(0),
        T::Cross => Some(1),
        T::Solid => Some(2),
    }
}

/// A choice's icon or wide sample by its name, as the menus take them (`&'static str`).
pub fn icon_name(name: &str) -> &'static str {
    static NAMES: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    NAMES
        .get_or_init(|| {
            choices()
                .into_iter()
                .flat_map(|c| [c.icon, c.preview])
                .map(|n| &*Box::leak(n.into_boxed_str()))
                .collect()
        })
        .iter()
        .find(|n| **n == name)
        .copied()
        .unwrap_or("hatch")
}
