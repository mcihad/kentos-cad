//! Yazı ve ölçü stilleri in the tools (docs/adr/0183 §4): Yazı, Çok satırlı
//! yazı, Metin dosyası yerleştir and every dimension tool write in the
//! style the session chose (`Memory::text_style`, `::dimension_style`),
//! Stil (S) in their prompt, its menu the project's styles and the styles'
//! window. A CAD project's interface only: a CBS project's tools write as
//! they always did (§4). The web's twin is `apps/web/src/tools/styleOption.ts`.

use kentos_contracts::{
    DimensionLook, DimensionStyleDef, ProjectSettings, STANDARD_DIMENSION_HEIGHT_MM,
    STANDARD_STYLE, TextFace, TextStyleDef, Workspace,
};
use kentos_domain::Uuid;

use crate::log::Level;
use crate::tool::{Context, Memory, OptionChoice};

/// Whether the tools show the styles: a CAD project's (docs/adr/0183 §4).
pub fn shown(settings: &ProjectSettings) -> bool {
    settings.project_type() == Some(Workspace::Cad)
}

/// A style's id as the session keeps it; none for an id that is no UUID.
fn uuid(id: &str) -> Option<Uuid> {
    Uuid::parse_str(id).ok()
}

/// The text style the session chose, when the project has it.
pub fn text_style<'s>(memory: &Memory, settings: &'s ProjectSettings) -> Option<&'s TextStyleDef> {
    let id = memory.text_style?;
    settings
        .text_styles
        .iter()
        .find(|s| uuid(&s.id) == Some(id))
}

/// The dimension style the session chose, when the project has it.
pub fn dimension_style<'s>(
    memory: &Memory,
    settings: &'s ProjectSettings,
) -> Option<&'s DimensionStyleDef> {
    let id = memory.dimension_style?;
    settings
        .dimension_styles
        .iter()
        .find(|s| uuid(&s.id) == Some(id))
}

/// What a tool shows of its style: whether it shows one, its name, and each
/// style's name with whether it is the chosen one (Standart first).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Seen {
    pub shown: bool,
    pub chosen: String,
    pub names: Vec<(String, bool)>,
}

impl Seen {
    fn of<'s>(shown: bool, chosen: Option<&'s str>, names: impl Iterator<Item = &'s str>) -> Seen {
        let mut all = vec![(STANDARD_STYLE.to_owned(), chosen.is_none())];
        all.extend(names.map(|n| (n.to_owned(), Some(n) == chosen)));
        Seen {
            shown,
            chosen: chosen.unwrap_or(STANDARD_STYLE).to_owned(),
            names: all,
        }
    }

    /// The text styles as a tool shows them.
    pub fn text(cx: &Context<'_>) -> Seen {
        let settings = cx.doc.settings();
        let chosen = text_style(cx.memory, settings).map(|s| s.name.as_str());
        Seen::of(
            shown(settings),
            chosen,
            settings.text_styles.iter().map(|s| s.name.as_str()),
        )
    }

    /// The dimension styles as a tool shows them.
    pub fn dimension(cx: &Context<'_>) -> Seen {
        let settings = cx.doc.settings();
        let chosen = dimension_style(cx.memory, settings).map(|s| s.name.as_str());
        Seen::of(
            shown(settings),
            chosen,
            settings.dimension_styles.iter().map(|s| s.name.as_str()),
        )
    }

    /// Stil's menu: Standart and the styles, then the window (`command`).
    pub fn choices(&self, window: &'static str, command: &'static str) -> Vec<OptionChoice> {
        let mut out: Vec<OptionChoice> = self
            .names
            .iter()
            .map(|(name, chosen)| OptionChoice {
                label: name.clone(),
                typed: name.clone(),
                icon: None,
                preview: None,
                checked: *chosen,
                command: None,
            })
            .collect();
        out.push(OptionChoice {
            label: window.to_owned(),
            typed: window.to_owned(),
            icon: Some(if command == TEXT_STYLES {
                "textStyle"
            } else {
                "dimensionStyle"
            }),
            preview: None,
            checked: false,
            command: Some(command),
        });
        out
    }
}

/// The styles' window's commands and their menu entries.
pub const TEXT_STYLES: &str = "style.textStyles";
pub const DIMENSION_STYLES: &str = "style.dimensionStyles";
pub const TEXT_STYLES_ENTRY: &str = "Yazı stilleri…";
pub const DIMENSION_STYLES_ENTRY: &str = "Ölçü stilleri…";

/// A typed name as a style's: trimmed, letters' case aside.
fn same_name(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

/// A typed or chosen text style (Stil): kept, the tool's height (when the
/// style fixes one) and width factor its; Standart takes the width factor
/// back to 1. A name the project has none of is said; false then.
pub fn take_text(typed: &str, cx: &mut Context<'_>) -> bool {
    if same_name(typed, STANDARD_STYLE) {
        cx.memory.text_style = None;
        cx.memory.text_width_factor = 1.0;
        return true;
    }
    let found = cx
        .doc
        .settings()
        .text_styles
        .iter()
        .find(|s| same_name(&s.name, typed))
        .map(|s| (uuid(&s.id), s.height, s.width_factor));
    match found {
        Some((Some(id), height, factor)) => {
            cx.memory.text_style = Some(id);
            if let Some(mm) = height {
                cx.memory.text_height_mm = mm;
            }
            cx.memory.text_width_factor = factor.unwrap_or(1.0);
            true
        }
        _ => {
            cx.say(
                Level::Warn,
                format!(
                    "“{}” adlı yazı stili yok. Stili menüden seçin ya da adını yazın; yeni stil Yazı stilleri penceresinde tanımlanır.",
                    typed.trim()
                ),
            );
            false
        }
    }
}

/// A typed or chosen dimension style (Stil), as `take_text`.
pub fn take_dimension(typed: &str, cx: &mut Context<'_>) -> bool {
    if same_name(typed, STANDARD_STYLE) {
        cx.memory.dimension_style = None;
        return true;
    }
    let found = cx
        .doc
        .settings()
        .dimension_styles
        .iter()
        .find(|s| same_name(&s.name, typed))
        .map(|s| uuid(&s.id));
    match found {
        Some(Some(id)) => {
            cx.memory.dimension_style = Some(id);
            true
        }
        _ => {
            cx.say(
                Level::Warn,
                format!(
                    "“{}” adlı ölçü stili yok. Stili menüden seçin ya da adını yazın; yeni stil Ölçü stilleri penceresinde tanımlanır.",
                    typed.trim()
                ),
            );
            false
        }
    }
}

/// The face a new text has: its style's in a CAD project, else none.
pub fn text_face(cx: &Context<'_>) -> TextFace {
    let settings = cx.doc.settings();
    match text_style(cx.memory, settings) {
        Some(s) if shown(settings) => s.face(),
        _ => TextFace::default(),
    }
}

/// A new multi-line text's width factor: its style's in a CAD project
/// (Çok satırlı yazı has no Genişlik of its own), else none.
pub fn text_width_factor(cx: &Context<'_>) -> Option<f64> {
    let settings = cx.doc.settings();
    text_style(cx.memory, settings)
        .filter(|_| shown(settings))
        .and_then(|s| s.width_factor)
}

/// A new dimension's look and value height: its style's in a CAD project,
/// else none and `standard` (the tool's own height, metres).
pub fn dimension_look(cx: &Context<'_>, standard: f64) -> (DimensionLook, f64) {
    let settings = cx.doc.settings();
    match dimension_style(cx.memory, settings) {
        Some(s) if shown(settings) => (s.look(), s.height_at(settings.plot_scale)),
        _ => (DimensionLook::default(), standard),
    }
}

/// Standart's value height at the project's scale, metres (the tools' 2.5 mm).
pub fn standard_dimension_height(settings: &ProjectSettings) -> f64 {
    STANDARD_DIMENSION_HEIGHT_MM / 1000.0 * settings.plot_scale
}
