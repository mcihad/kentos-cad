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

use crate::Vec2;
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
    /// Boşluk, in the line work (docs/adr/0329 §3).
    Gap,
    /// Başlangıç's point, by a click (docs/adr/0331 §2).
    Origin,
    /// Özellik al: a hatch whose pattern the next hatches take (docs/adr/0331 §2).
    Inherit,
}

/// Başlangıç's rules (docs/adr/0331 §2), `Memory::hatch_origin`'s order: their words and what the command takes.
pub const ORIGINS: [&str; 7] = ["dünya", "sol alt", "sağ alt", "sol üst", "sağ üst", "orta", "nokta"];

/// The project's own patterns' names (docs/adr/0331 §1), in their order.
pub fn user_names(cx: &Context<'_>) -> Vec<String> {
    cx.doc
        .settings()
        .hatch_patterns
        .iter()
        .map(|d| d.name.clone())
        .collect()
}

/// The project pattern chosen, by its name; none: the library's choice.
pub fn user_of<'a>(m: &Memory, user: &'a [String]) -> Option<&'a str> {
    m.hatch_user
        .and_then(|k| user.get(usize::from(k)))
        .map(String::as_str)
}

/// Whether the pattern chosen draws lines (Ölçek, Başlangıç and Ölçekli read).
pub fn lined(m: &Memory, user: &[String]) -> bool {
    user_of(m, user).is_some()
        || !matches!(choice(m).kind, Kind::Solid | Kind::Gradient(_))
}

/// Başlangıç as the command takes it.
pub fn origin_spec(m: &Memory) -> Option<kentos_contracts::HatchOrigin> {
    use kentos_contracts::{BoxCorner as C, HatchOrigin as O};
    let corner = |corner| O::Corner {
        corner,
        across: m.hatch_origin_across,
    };
    match m.hatch_origin {
        1 => Some(corner(C::BottomLeft)),
        2 => Some(corner(C::BottomRight)),
        3 => Some(corner(C::TopLeft)),
        4 => Some(corner(C::TopRight)),
        5 => Some(corner(C::Center)),
        6 => m.hatch_origin_at.map(|p| O::Point {
            at: kentos_contracts::Vec2 { x: p.x, y: p.y },
        }),
        _ => None,
    }
}

/// How the islands are taken as Adalar says (docs/adr/0329 §1): taranmaz is Dış, iç içe Normal, taranır Yok say.
pub fn island_style(m: &Memory) -> Option<kentos_contracts::IslandStyle> {
    (m.hatch_islands && m.hatch_nested).then_some(kentos_contracts::IslandStyle::Normal)
}

/// Adalar's word.
fn islands_word(m: &Memory) -> &'static str {
    match (m.hatch_islands, m.hatch_nested) {
        (false, _) => "taranır",
        (true, true) => "iç içe",
        (true, false) => "taranmaz",
    }
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

/// The pattern as Tarama's command takes it (docs/adr/0254 §7): the
/// session's choice by its name, and its options.
pub fn spec(m: &Memory, user: &[String]) -> kentos_contracts::HatchPatternSpec {
    let lined = lined(m, user);
    kentos_contracts::HatchPatternSpec {
        name: user_of(m, user).map_or_else(|| choice(m).name, str::to_owned),
        scale: Some(m.hatch_scale),
        angle: Some(m.hatch_angle),
        second_color: Some(colour_text(m)),
        inverted: Some(m.hatch_inverted),
        basis: (lined && m.hatch_world).then_some(kentos_contracts::ScaleBasis::World),
        origin: if lined { origin_spec(m) } else { None },
        annotative: (lined && m.hatch_annotative && !m.hatch_world).then_some(true),
    }
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
pub fn with_pattern_options(prompt: Prompt, m: &Memory, user: &[String]) -> Prompt {
    let c = choice(m);
    let own = user_of(m, user);
    let gradient = own.is_none() && matches!(c.kind, Kind::Gradient(_));
    let solid = own.is_none() && c.kind == Kind::Solid;
    let mut prompt = prompt.option_with("Desen", "D", own.map_or(c.name, str::to_owned));
    if gradient {
        prompt = prompt
            .option_with("İkinci renk", "R", colour_text(m))
            .option_with("Ters", "T", on(m.hatch_inverted));
    } else if !solid {
        prompt = prompt.option_with("Ölçek", "Ö", js_number(m.hatch_scale));
    }
    if !solid {
        prompt = prompt.option_with("Açı", "Ç", format!("{}°", js_number(m.hatch_angle)));
    }
    // docs/adr/0331: what Ölçek is of, where the lines are anchored, Ölçekli; and Özellik al.
    if lined(m, user) {
        prompt = prompt
            .option_with("Ölçek tabanı", "E", if m.hatch_world { "çizimde" } else { "kâğıtta" })
            .option_with("Başlangıç", "S", ORIGINS[usize::from(m.hatch_origin.min(6))])
            .option_if(!m.hatch_world, "Ölçekli", "L", on(m.hatch_annotative));
    }
    prompt.option("Özellik al", "Z")
}

/// Adalar, İlişkili (where the hatch can follow its objects) and Yazılar.
pub fn with_region_options(prompt: Prompt, m: &Memory, tie: bool) -> Prompt {
    prompt
        .option_with("Adalar", "A", islands_word(m))
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
        Asking::Gap => Prompt::new(
            label,
            "kapatılacak en büyük boşluğu metre olarak yazın (0: boşluk kapatılmaz)",
        ),
        Asking::Colour => Prompt::new(
            label,
            "ikinci rengi yazın (#RRGGBB ya da renk adı) ya da menüden seçin",
        )
        .option("İkinci renk", "R"),
        Asking::Origin => Prompt::new(label, "desenin başlangıç noktasına tıklayın ya da Y,X yazın"),
        Asking::Inherit => Prompt::new(label, "özelliklerini alacağınız taramaya tıklayın"),
    }
}

/// A click while Başlangıç's point or Özellik al is asked: whether it answered (docs/adr/0331 §2).
pub fn clicked(asking: &mut Option<Asking>, at: Vec2, raw: Vec2, cx: &mut Context<'_>) -> bool {
    match *asking {
        Some(Asking::Origin) => {
            cx.memory.hatch_origin = 6;
            cx.memory.hatch_origin_at = Some(at);
            *asking = None;
            true
        }
        Some(Asking::Inherit) => {
            let hatch = crate::selectable::candidates(cx, raw)
                .into_iter()
                .find_map(|s| match cx.doc.get(s) {
                    Some(kentos_contracts::Entity::Hatch(h)) => Some(h.clone()),
                    _ => None,
                });
            match hatch {
                Some(h) => {
                    let user = user_names(cx);
                    let plot = cx.doc.settings().plot_scale;
                    inherit(&h, cx.memory, &user, plot);
                    *asking = None;
                    cx.say(Level::Success, "Taramanın deseni, ölçeği, açısı ve başlangıcı alındı.");
                }
                None => cx.say(Level::Warn, "Özelliklerini alacağınız bir taramaya tıklayın."),
            }
            true
        }
        _ => false,
    }
}

/// The options a hatch was made with, as near as its pattern says them (AutoCAD's Inherit properties, docs/adr/0331
/// §2): its pattern (the project's by name first), Ölçek and Açı on paper at its own scale, its origin, Ölçekli.
pub fn inherit(h: &kentos_contracts::HatchEntity, m: &mut Memory, user: &[String], plot_scale: f64) {
    use kentos_contracts::HatchPatternType as T;
    let p = &h.pattern;
    let at = h.annotative.as_ref().map_or(plot_scale, |a| a.at);
    let own = (p.kind == T::Pattern)
        .then_some(p.name.as_deref())
        .flatten()
        .and_then(|n| user.iter().position(|u| kentos_contracts::same_hatch_pattern_name(u, n)));
    m.hatch_user = own.and_then(|k| u16::try_from(k).ok());
    if own.is_none()
        && let Some(k) = choice_of(p)
    {
        m.hatch_choice = k;
    }
    let metres = at / 1000.0;
    let scale = match p.kind {
        T::Pattern => p.scale.unwrap_or(metres) / metres,
        T::Lines | T::Cross => {
            p.spacing / (kentos_geometry_core::tools::hatch::USER_SPACING_MM * metres)
        }
        _ => 1.0,
    };
    if scale.is_finite() && scale > 0.0 && scale <= MOST_SCALE {
        m.hatch_scale = scale;
    }
    m.hatch_angle = match p.kind {
        T::Lines | T::Cross => p.angle - kentos_geometry_core::tools::hatch::USER_ANGLE,
        T::Solid => 0.0,
        _ => p.angle,
    };
    if let Some(g) = &p.gradient {
        m.hatch_color2 = u32::from_str_radix(g.color2.trim_start_matches('#'), 16).unwrap_or(0xFF_FF_FF);
        m.hatch_inverted = g.inverted;
    }
    m.hatch_world = false;
    m.hatch_annotative = h.annotative.is_some();
    match p.origin {
        Some(o) => {
            m.hatch_origin = 6;
            m.hatch_origin_at = Some(Vec2::new(o.x, o.y));
        }
        None => m.hatch_origin = 0,
    }
}

/// The options' keys: whether `key` was one of them (a key that asks for a
/// value starts asking it). `tie`: İlişkili is offered.
pub fn option(key: &str, tie: bool, asking: &mut Option<Asking>, cx: &mut Context<'_>) -> bool {
    let user = user_names(cx);
    let own = user_of(cx.memory, &user).is_some();
    let kind = choice(cx.memory).kind;
    let gradient = !own && matches!(kind, Kind::Gradient(_));
    let solid = !own && kind == Kind::Solid;
    let lined = lined(cx.memory, &user);
    let m = &mut *cx.memory;
    match key {
        // Desen: the library's next; a project pattern chosen goes back to the library's.
        "D" if own => m.hatch_user = None,
        "D" => m.hatch_choice = (m.hatch_choice + 1) % choices().len(),
        "Ö" if !gradient && !solid => *asking = Some(Asking::Scale),
        "Ç" if !solid => *asking = Some(Asking::Angle),
        "E" if lined => {
            m.hatch_world = !m.hatch_world;
            if m.hatch_world {
                m.hatch_annotative = false;
            }
        }
        // Başlangıç: dünya → sol alt → … → orta → nokta (asked by a click) → dünya.
        "S" if lined => {
            m.hatch_origin = (m.hatch_origin + 1) % 7;
            if m.hatch_origin == 6 {
                *asking = Some(Asking::Origin);
            }
        }
        "L" if lined && !m.hatch_world => m.hatch_annotative = !m.hatch_annotative,
        "Z" => *asking = Some(Asking::Inherit),
        "R" if gradient => *asking = Some(Asking::Colour),
        "T" if gradient => m.hatch_inverted = !m.hatch_inverted,
        // taranmaz → taranır → iç içe → taranmaz.
        "A" => {
            (m.hatch_islands, m.hatch_nested) = match (m.hatch_islands, m.hatch_nested) {
                (true, false) => (false, false),
                (false, _) => (true, true),
                (true, true) => (true, false),
            }
        }
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
        Asking::Gap => match parse_number(t) {
            Some(n) if (0.0..=kentos_contracts::MAX_HATCH_GAP).contains(&n) => {
                cx.memory.hatch_gap = n;
                *asking = None;
            }
            _ => cx.say(
                Level::Warn,
                format!("Boşluk 0 ile 1000 m arasında bir sayı olmalı; “{t}” yazıldı."),
            ),
        },
        Asking::Origin => match cx.typed_point(t, None, None) {
            Some(p) => {
                cx.memory.hatch_origin = 6;
                cx.memory.hatch_origin_at = Some(p);
                *asking = None;
            }
            None => cx.say(Level::Warn, format!("Başlangıç noktası Y,X olarak yazılmalı; “{t}” yazıldı.")),
        },
        Asking::Inherit => cx.say(Level::Warn, "Özelliklerini alacağınız taramaya tıklayın; Esc vazgeçer."),
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
    // A project pattern of the name first, as the command finds it (docs/adr/0331 §1).
    let user = user_names(cx);
    if let Some(k) = user
        .iter()
        .position(|u| kentos_contracts::same_hatch_pattern_name(u, js_trim(text)))
    {
        cx.memory.hatch_user = u16::try_from(k).ok();
        return true;
    }
    match choice_named(text) {
        Some(k) => {
            cx.memory.hatch_choice = k;
            cx.memory.hatch_user = None;
            true
        }
        None => false,
    }
}

/// Desen's and İkinci renk's menus.
pub fn choices_of(key: &str, m: &Memory, user: &[String]) -> Vec<OptionChoice> {
    let own = user_of(m, user);
    match key {
        "D" => choices()
            .into_iter()
            .enumerate()
            .map(|(k, c)| OptionChoice {
                label: c.label,
                typed: c.typed,
                icon: Some(icon_name(&c.icon)),
                preview: Some(icon_name(&c.preview)),
                checked: own.is_none() && k == m.hatch_choice,
                command: None,
            })
            // The project's own, after the library's (docs/adr/0331 §1).
            .chain(user.iter().map(|name| OptionChoice {
                label: format!("{name} (proje)"),
                typed: name.clone(),
                icon: Some("hatchPatterns"),
                preview: None,
                checked: own == Some(name.as_str()),
                command: None,
            }))
            .collect(),
        "S" => ["Dünya başlangıcı", "Sol alt", "Sağ alt", "Sol üst", "Sağ üst", "Orta", "Nokta…"]
            .iter()
            .zip(ORIGINS)
            .enumerate()
            .map(|(k, (label, typed))| OptionChoice {
                label: (*label).to_owned(),
                typed: typed.to_owned(),
                icon: None,
                preview: None,
                checked: usize::from(m.hatch_origin) == k,
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
        "S" => match ORIGINS.iter().position(|o| *o == typed) {
            Some(k) => {
                cx.memory.hatch_origin = k as u8;
                *asking = (k == 6).then_some(Asking::Origin);
                true
            }
            None => false,
        },
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
