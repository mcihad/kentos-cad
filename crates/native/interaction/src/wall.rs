//! Duvar (docs/adr/0339 §6): the path tool's wall (`tool.wall`), drawn as a polyline is, with its arcs, typed points,
//! locks and Kapat (K), and the wall's own options:
//!
//! - Genişlik (E): the next number typed is the width; a style whose components fix it keeps its own;
//! - Hiza (H): Sol, Orta or Sağ, where the body lies across the path as drawn;
//! - Öteleme (Ö, or O): the next number typed moves the base line to the path's left (minus: to its right);
//! - Stil (S): Yok, the project's wall styles and KentOS's own, then the styles' window; a style chosen gives the
//!   width its new walls take.
//!
//! The options stay for as long as the app lives (`Memory::wall_*`). The product command `cad.walls.create` does the
//! work: its plan is the preview (the faces and the boundaries between components over the body, lightly filled), its
//! execute the write, one undo step “Duvar” on the active layer in the current colour, weight and line type.

use kentos_contracts::{
    CommandResult, NewWall, WallJustify, WallStyleDef, WallsCreate, builtin_wall_styles,
    fixed_width,
};
use kentos_geometry_core::geom::wall;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, walls_create};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, wire};
use crate::prompt::Prompt;
use crate::tool::{Area, Context, Memory, OptionChoice, Stroke, Tone};

/// The wall tool's id: its command is `tool.wall`.
pub const ID: &str = "wall";
pub const LABEL: &str = "Duvar";
/// The wall styles' window and its menu entry.
pub const WALL_STYLES: &str = "arch.wallStyles";
pub const WALL_STYLES_ENTRY: &str = "Duvar stilleri…";
/// Stil's entry for a wall of one plain body.
pub const NO_STYLE: &str = "Yok";

/// A value the wall tool waits for after its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    Width,
    Offset,
    /// A style's name, typed after S: words (Space types a space).
    Style,
}

/// A style's id as the session keeps it (`Memory` is `Copy`): its FNV-1a hash.
pub fn style_key(id: &str) -> u64 {
    id.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The styles the tool offers: the project's, then KentOS's own the project has not taken.
pub fn offered(settings: &kentos_contracts::ProjectSettings) -> Vec<WallStyleDef> {
    let mut all = settings.wall_styles.clone();
    for s in builtin_wall_styles() {
        if !all.iter().any(|p| p.id == s.id) {
            all.push(s);
        }
    }
    all
}

/// The style the session chose, when one of [`offered`] still has its id.
pub fn chosen(
    memory: &Memory,
    settings: &kentos_contracts::ProjectSettings,
) -> Option<WallStyleDef> {
    chosen_in(memory, &offered(settings)).cloned()
}

/// [`chosen`] among the styles offered.
pub fn chosen_in<'s>(memory: &Memory, offered: &'s [WallStyleDef]) -> Option<&'s WallStyleDef> {
    let key = memory.wall_style?;
    offered.iter().find(|s| style_key(&s.id) == key)
}

/// Hiza typed alone: the next justification, Sol, Orta, Sağ and round.
pub fn next_justify(cx: &mut Context<'_>) {
    cx.memory.wall_justify = match cx.memory.wall_justify {
        WallJustify::Left => WallJustify::Center,
        WallJustify::Center => WallJustify::Right,
        WallJustify::Right => WallJustify::Left,
    };
}

/// The width the next wall takes: a style's that fixes it, else the session's.
pub fn width(memory: &Memory, style: Option<&WallStyleDef>) -> f64 {
    style
        .and_then(|s| fixed_width(&s.components))
        .unwrap_or(memory.wall_width)
}

/// The wall's chips: its width, justification, offset and style, as they stand.
pub fn chips(
    prompt: Prompt,
    memory: &Memory,
    format: &Format,
    style: Option<&WallStyleDef>,
) -> Prompt {
    prompt
        .option_with("Genişlik", "E", format.length(width(memory, style)))
        .option_with("Hiza", "H", memory.wall_justify.label())
        .option_with("Öteleme", "Ö", format.length(memory.wall_offset))
        .option_with(
            "Stil",
            "S",
            style.map_or(NO_STYLE.to_owned(), |s| s.name.clone()),
        )
}

/// The prompt while a value is waited for.
pub fn asked(ask: Ask) -> Prompt {
    match ask {
        Ask::Width => Prompt::new(LABEL, "duvarın genişliğini yazın"),
        Ask::Offset => Prompt::new(
            LABEL,
            "taban çizgisinin ötelemesini yazın (artı: yolun solu, eksi: sağı)",
        ),
        Ask::Style => Prompt::new(LABEL, "duvar stilinin adını yazın (Yok: stilsiz)"),
    }
    .option("Geri", "G")
}

/// The key of a value to wait for; none for another key.
pub fn ask_of(key: &str) -> Option<Ask> {
    match key {
        "E" => Some(Ask::Width),
        "Ö" | "O" => Some(Ask::Offset),
        "S" => Some(Ask::Style),
        _ => None,
    }
}

/// A number typed for `ask` (`metres`: in the project's unit, or its own); said when it does not hold.
pub fn typed(ask: Ask, metres: f64, cx: &mut Context<'_>) {
    match ask {
        Ask::Width if metres > 0.0 && metres <= 100.0 => cx.memory.wall_width = metres,
        Ask::Width => cx.say(
            Level::Warn,
            "Duvarın genişliği sıfırdan büyük ve en çok 100 m olmalı.",
        ),
        Ask::Offset if metres.is_finite() && metres.abs() <= 100.0 => {
            cx.memory.wall_offset = metres
        }
        Ask::Offset => cx.say(Level::Warn, "Öteleme en çok 100 m olmalı."),
        Ask::Style => {}
    }
}

/// Hiza's and Stil's menus.
pub fn choices(key: &str, memory: &Memory, offered: &[WallStyleDef]) -> Vec<OptionChoice> {
    let choice = |label: String, icon: Option<&'static str>, checked: bool| OptionChoice {
        typed: label.clone(),
        label,
        icon,
        preview: None,
        checked,
        command: None,
    };
    match key {
        "H" => WallJustify::ALL
            .iter()
            .map(|&j| {
                let icon = match j {
                    WallJustify::Left => "wallLeft",
                    WallJustify::Center => "wallCenter",
                    WallJustify::Right => "wallRight",
                };
                choice(j.label().to_owned(), Some(icon), j == memory.wall_justify)
            })
            .collect(),
        "S" => {
            let mut out = vec![choice(
                NO_STYLE.to_owned(),
                None,
                memory.wall_style.is_none(),
            )];
            out.extend(offered.iter().map(|s| {
                choice(
                    s.name.clone(),
                    None,
                    Some(style_key(&s.id)) == memory.wall_style,
                )
            }));
            out.push(OptionChoice {
                label: WALL_STYLES_ENTRY.to_owned(),
                typed: WALL_STYLES_ENTRY.to_owned(),
                icon: Some("wallStyles"),
                preview: None,
                checked: false,
                command: Some(WALL_STYLES),
            });
            out
        }
        _ => Vec::new(),
    }
}

/// Hiza or Stil chosen (or typed after its key): taken, else said; false for another key.
pub fn choose(key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
    let folded = typed.trim().to_lowercase();
    match key {
        "H" => {
            match WallJustify::ALL
                .iter()
                .find(|j| j.label().to_lowercase() == folded)
            {
                Some(&j) => cx.memory.wall_justify = j,
                None => cx.say(Level::Warn, "Hiza Sol, Orta ya da Sağ olabilir."),
            }
            true
        }
        "S" => {
            if folded == NO_STYLE.to_lowercase() {
                cx.memory.wall_style = None;
                return true;
            }
            let found = offered(cx.doc.settings())
                .into_iter()
                .find(|s| s.name.to_lowercase() == folded || s.id == typed.trim());
            match found {
                Some(s) => {
                    cx.memory.wall_style = Some(style_key(&s.id));
                    cx.memory.wall_width = s.new_width();
                }
                None => cx.say(
                    Level::Warn,
                    format!(
                        "“{}” adlı duvar stili yok. Stili menüden seçin ya da adını yazın; yeni stil Duvar stilleri penceresinde tanımlanır.",
                        typed.trim()
                    ),
                ),
            }
            true
        }
        _ => false,
    }
}

/// The command's input: the path as drawn (one bulge per edge, `closed` with the closing one) and the session's
/// options, on the active layer.
pub fn input(pts: &[Vec2], bulges: &[f64], closed: bool, cx: &Context<'_>) -> WallsCreate {
    let style = chosen(cx.memory, cx.doc.settings());
    let fixed = style
        .as_ref()
        .and_then(|s| fixed_width(&s.components))
        .is_some();
    let m = *cx.memory;
    WallsCreate {
        layer_id: cx.doc.layers().active().to_owned(),
        walls: vec![NewWall {
            pts: pts.iter().copied().map(wire).collect(),
            bulges: bulges.iter().any(|b| *b != 0.0).then(|| bulges.to_vec()),
            closed,
            width: (!fixed).then_some(m.wall_width),
            justify: (m.wall_justify != WallJustify::Center).then_some(m.wall_justify),
            offset: (m.wall_offset != 0.0).then_some(m.wall_offset),
            style: style.map(|s| s.id),
            caps: None,
            height: None,
            arch: None,
            attrs: cx.template_attrs(None),
            label: cx.template_label(),
        }],
        color: cx.draft.color_text(),
        line_weight: cx.draft.line_weight,
        line_type: cx.draft.line_type,
        expected_revision: None,
    }
}

/// What the preview draws of the wall the path would make: the body lightly filled, its faces and the boundaries
/// between its components; or why it cannot be drawn.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Drawn {
    pub areas: Vec<Area>,
    pub strokes: Vec<Stroke>,
    pub why: Option<String>,
}

/// The command's plan of the path to the cursor, as the preview draws it.
pub fn drawn(pts: &[Vec2], bulges: &[f64], closed: bool, cx: &Context<'_>) -> Drawn {
    if pts.len() < 2 {
        return Drawn::default();
    }
    let plan = walls_create::plan_in(cx.doc, &input(pts, bulges, closed, cx));
    let entity = match plan {
        CommandResult::Completed { output, .. } => output.walls.into_iter().next(),
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => {
            return Drawn {
                why: Some(error.message),
                ..Drawn::default()
            };
        }
        CommandResult::Queued { .. } | CommandResult::Cancelled => None,
    };
    let mut out = Drawn::default();
    if let Some(e) = &entity {
        add(e, &mut out);
    }
    out
}

/// A planned wall as the preview draws it, added to `out`: its body lightly filled, its faces, the boundaries between
/// its components dashed and an open wall's caps; a wall that cannot be drawn says why.
pub fn add(e: &kentos_contracts::Entity, out: &mut Drawn) {
    let kentos_geometry_core::entity::Shape::Wall {
        pts,
        bulges,
        wall: spec,
    } = shape(e)
    else {
        return;
    };
    let lines = match wall::lines(&pts, bulges.as_deref(), &spec) {
        Ok(lines) => lines,
        Err(why) => {
            out.why = Some(why);
            return;
        }
    };
    let last = lines.len() - 1;
    if let Ok((outer, holes)) = wall::body(&pts, bulges.as_deref(), &spec) {
        let mut rings = vec![outer];
        rings.extend(holes.unwrap_or_default());
        out.areas.push(Area {
            rings,
            fill: 0.22,
            width: 0.0,
            dash: None,
            fill_tone: Tone::Accent,
        });
    }
    for (i, line) in lines.iter().enumerate() {
        let face = i == 0 || i == last;
        let stroke = if face {
            Stroke::solid(line.outline(spec.closed), spec.closed).width(1.5)
        } else {
            Stroke::dashed(line.outline(spec.closed), spec.closed, [4.0, 3.0]).width(1.0)
        };
        out.strokes.push(Stroke {
            tone: Tone::Accent,
            ..stroke
        });
    }
    // An open wall's ends across it, as the caps close them.
    if !spec.closed && !spec.caps_open {
        for first in [true, false] {
            let end = |l: &wall::Line| {
                if first {
                    l.pts[0]
                } else {
                    l.pts[l.pts.len() - 1]
                }
            };
            out.strokes.push(Stroke {
                tone: Tone::Accent,
                ..Stroke::solid(vec![end(&lines[0]), end(&lines[last])], false).width(1.5)
            });
        }
    }
}

/// Writes the wall through `cad.walls.create`; whether it was written.
pub fn write(pts: &[Vec2], bulges: &[f64], closed: bool, cx: &mut Context<'_>) -> bool {
    let input = input(pts, bulges, closed, cx);
    let result = walls_create::execute(&mut ExecutionContext::new(cx.doc), input);
    match points::written(result, cx) {
        Some(out) => {
            for id in &out.styles_added {
                let name = cx
                    .doc
                    .settings()
                    .wall_styles
                    .iter()
                    .find(|s| &s.id == id)
                    .map(|s| s.name.clone());
                if let Some(name) = name {
                    cx.say(
                        Level::Info,
                        format!("“{name}” duvar stili projeye eklendi."),
                    );
                }
            }
            true
        }
        None => false,
    }
}
