//! `cad.fonts.report` v1 (docs/adr/0324 §5): each text style's typeface, which of KentOS's faces it is drawn in and
//! how (the core's `text::fonts::resolve`), the names asked about, the faces a sheet's PDF embeds. It writes nothing;
//! a style drawn in a substitute is a `font_substituted` warning, so the change is never silent. The shared cases
//! are `fixtures/commands/v1/cad.fonts.report.json` (`scripts/fixtures/font_report_cases.py`).

use std::collections::{BTreeSet, HashMap};

use kentos_contracts::{
    CommandResult, CommandWarning, DrawingFont, Entity, FontMatch, FontsReport, FontsReported,
    MAX_FONT_NAME, NamedFont, StyleFont,
};
use kentos_domain::Document;
use kentos_geometry_core::text::fonts::{self, FontMatch as Core};

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;

fn contract(how: Core) -> FontMatch {
    match how {
        Core::Own => FontMatch::Own,
        Core::Equivalent => FontMatch::Equivalent,
        Core::Shx => FontMatch::Shx,
        Core::Missing => FontMatch::Missing,
    }
}

fn check(input: &FontsReport) -> Result<(), Stop> {
    for (i, name) in input.names.iter().enumerate() {
        if name.trim().is_empty() {
            return Err(Stop::failed(error(
                codes::EMPTY_NAME,
                format!(
                    "{}. yazı tipi adı boş. Bir dosya ya da aile adı yazın (arial.ttf, romans.shx).",
                    i + 1
                ),
                Some(format!("names[{i}]")),
            )));
        }
        if name.chars().count() > MAX_FONT_NAME {
            return Err(Stop::failed(error(
                codes::INVALID_NAME,
                format!(
                    "{}. yazı tipi adı {MAX_FONT_NAME} karakterden uzun. Kısaltın.",
                    i + 1
                ),
                Some(format!("names[{i}]")),
            )));
        }
    }
    Ok(())
}

/// Checks `input`.
pub fn validate(_cx: &ExecutionContext<'_>, input: &FontsReport) -> CommandResult<()> {
    match check(input) {
        Ok(()) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The report; the same answer as [`execute`].
pub fn plan(cx: &ExecutionContext<'_>, input: &FontsReport) -> CommandResult<FontsReported> {
    answer(cx.doc, input)
}

/// The report. Writes nothing.
pub fn execute(cx: &mut ExecutionContext<'_>, input: FontsReport) -> CommandResult<FontsReported> {
    answer(cx.doc, &input)
}

fn answer(doc: &Document, input: &FontsReport) -> CommandResult<FontsReported> {
    if let Err(stop) = check(input) {
        return stop.into();
    }
    let (output, warnings) = report(doc, input);
    CommandResult::Completed { output, warnings }
}

/// The report of `doc` (its checks passed) and its warnings.
pub fn report(doc: &Document, input: &FontsReport) -> (FontsReported, Vec<CommandWarning>) {
    let settings = doc.settings();
    let project = settings.drawing_font.unwrap_or(DrawingFont::Barlow);
    // The texts by their style, and the faces everything is drawn in.
    let mut used: HashMap<&str, u32> = HashMap::new();
    let mut faces: BTreeSet<usize> = BTreeSet::new();
    let place = |f: DrawingFont| DrawingFont::ALL.iter().position(|x| *x == f).unwrap_or(0);
    for e in doc.entities() {
        match e {
            Entity::Text(t) => {
                if let Some(id) = t.face.text_style.as_deref() {
                    *used.entry(id).or_default() += 1;
                }
                faces.insert(place(t.face.font.unwrap_or(project)));
            }
            Entity::Dimension(d) => {
                faces.insert(place(d.look.font.unwrap_or(project)));
            }
            Entity::Leader(_) => {
                faces.insert(place(project));
            }
            Entity::Table(t) => {
                faces.insert(place(t.face.font.unwrap_or(project)));
            }
            _ => {}
        }
    }
    let mut warnings = Vec::new();
    let styles: Vec<StyleFont> = settings
        .text_styles
        .iter()
        .enumerate()
        .map(|(i, s)| {
            // The file it named resolved by the core's rule; none named, its own face.
            let (named, how) = match s.font_file.as_deref() {
                Some(file) => {
                    let (id, how) = fonts::resolve(file);
                    (id, contract(how))
                }
                None => (Some(s.font.id()), FontMatch::Own),
            };
            // Drawn in another face than the one named: said, never silently.
            if named != Some(s.font.id()) {
                let what = match how {
                    FontMatch::Shx => "AutoCAD'in SHX çizgi yazı tipi, KentOS okumaz",
                    FontMatch::Missing => "KentOS'ta yok",
                    FontMatch::Own | FontMatch::Equivalent => {
                        "stil başka bir yazı tipine çevrilmiş"
                    }
                };
                warnings.push(CommandWarning {
                    code: codes::FONT_SUBSTITUTED.into(),
                    message: format!(
                        "“{}” stili {} yazı tipini adlandırıyor ({what}); yazıları {} ile çizilir.",
                        s.name,
                        s.font_file.as_deref().unwrap_or_default(),
                        s.font.label()
                    ),
                    path: Some(format!("styles[{i}]")),
                });
            }
            StyleFont {
                style: s.id.clone(),
                name: s.name.clone(),
                file: s.font_file.clone(),
                face: s.font,
                how,
                texts: used.get(s.id.as_str()).copied().unwrap_or(0),
            }
        })
        .collect();
    let names = input
        .names
        .iter()
        .map(|name| {
            let (id, how) = fonts::resolve(name);
            NamedFont {
                name: name.clone(),
                face: id.and_then(DrawingFont::from_id),
                how: contract(how),
            }
        })
        .collect();
    let output = FontsReported {
        project,
        styles,
        names,
        faces: faces.into_iter().map(|i| DrawingFont::ALL[i]).collect(),
        revision: doc.revision().to_string(),
    };
    (output, warnings)
}
