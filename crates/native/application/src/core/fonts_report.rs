//! `cad.fonts.report` v1 (docs/adr/0324 §5): each text style's typeface, which face it is drawn in and how (the
//! core's `text::fonts::resolve`), the names asked about, the faces a sheet's PDF embeds, the typefaces a person can
//! pick and the fonts folders' files (docs/adr/9313). It writes nothing; a style drawn in a substitute, and a folder
//! family this program's folders do not hold, are `font_substituted` warnings, so the change is never silent. The
//! shared cases are `fixtures/commands/v1/cad.fonts.report.json` (`scripts/fixtures/font_report_cases.py`).

use std::collections::HashMap;

use kentos_commands::{
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
        Core::Folder => FontMatch::Folder,
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
    let mut faces: Vec<DrawingFont> = Vec::new();
    let mut face = |f: DrawingFont| {
        if !faces.contains(&f) {
            faces.push(f);
        }
    };
    for e in doc.entities() {
        match e {
            Entity::Text(t) => {
                if let Some(id) = t.face.text_style.as_deref() {
                    *used.entry(id).or_default() += 1;
                }
                face(t.face.font.unwrap_or(project));
            }
            Entity::Dimension(d) => face(d.look.font.unwrap_or(project)),
            Entity::Leader(_) => face(project),
            Entity::Table(t) => face(t.face.font.unwrap_or(project)),
            _ => {}
        }
    }
    // The contract's order: KentOS's own faces, then the folder families by name.
    faces.sort_by_key(|f| (DrawingFont::ALL.iter().position(|x| x == f).unwrap_or(usize::MAX), f.label()));
    let mut warnings = Vec::new();
    // A folder family this program's folders do not hold is drawn in Arial (docs/adr/9313): said, never silently.
    for f in faces.iter().filter(|f| !crate::font_folders::drawable(**f)) {
        warnings.push(CommandWarning {
            code: codes::FONT_SUBSTITUTED.into(),
            message: format!(
                "“{}” yazı tipi bu bilgisayarın yazı tipi klasörlerinde yok; yazıları Arial ile çizilir. Ailenin dosyasını kullanıcının yazı tipi klasörüne koyup programı yeniden başlatın.",
                f.label()
            ),
            path: Some("faces".to_owned()),
        });
    }
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
            // Its embedded SHX font draws it (ADR 1125): not a substitute.
            let embedded = how == FontMatch::Shx
                && crate::shx_text::Fonts::of(s, doc.styles()).is_ok();
            // Drawn in another face than the one named: said, never silently.
            if named != Some(s.font.id()) && !embedded {
                let what = match how {
                    FontMatch::Shx => "SHX çizgi yazı tipi; projeye eklenmemiş ya da okunamıyor",
                    FontMatch::Missing => "KentOS'ta yok",
                    FontMatch::Own | FontMatch::Equivalent | FontMatch::Folder => {
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
                embedded,
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
                face: id.and_then(DrawingFont::from_name),
                how: contract(how),
            }
        })
        .collect();
    let output = FontsReported {
        project,
        styles,
        names,
        faces,
        choices: crate::font_folders::choices(None),
        files: crate::font_folders::listing().to_vec(),
        revision: doc.revision().to_string(),
    };
    (output, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_commands::DocumentSnapshotV2;

    /// The styles fixture, its project in `font` and its first text in the project's face.
    fn drawing(font: &str) -> Document {
        let mut raw: serde_json::Value =
            serde_json::from_str(include_str!("../../../../../fixtures/kcad/v2/styles.json")).expect("the fixture");
        raw["settings"]["drawingFont"] = serde_json::Value::from(font);
        if let Some(text) = raw["entities"]
            .as_array_mut()
            .and_then(|es| es.iter_mut().find(|e| e["kind"] == "text"))
            .and_then(serde_json::Value::as_object_mut)
        {
            text.remove("font");
            text.remove("textStyle");
            text.remove("bold");
            text.remove("italic");
        }
        let snapshot: DocumentSnapshotV2 = serde_json::from_value(raw).expect("a snapshot");
        Document::from_snapshot_v2(snapshot).expect("opens")
    }

    /// The typefaces a person can pick start with KentOS's own nine (docs/adr/9313).
    #[test]
    fn the_choices_start_with_the_own_faces() {
        let (out, warnings) = report(&drawing("arial"), &FontsReport::default());
        assert_eq!(&out.choices[..9], &DrawingFont::CHOICES);
        assert!(out.faces.contains(&DrawingFont::Arial), "{:?}", out.faces);
        assert!(warnings.iter().all(|w| !w.message.contains("klasörlerinde yok")), "{warnings:?}");
    }

    /// A folder family this program's folders do not hold is drawn in Arial, and said (docs/adr/9313 §4).
    #[test]
    fn a_family_this_program_has_not_got_is_said() {
        let kurum = DrawingFont::from_name("Kurum Sans").expect("a family");
        let (out, warnings) = report(&drawing("Kurum Sans"), &FontsReport::default());
        assert_eq!(out.project, kurum);
        assert!(out.faces.contains(&kurum), "{:?}", out.faces);
        // Own faces first, the families after them.
        assert_eq!(out.faces.last(), Some(&kurum));
        let said: Vec<&CommandWarning> = warnings.iter().filter(|w| w.message.contains("“Kurum Sans”")).collect();
        assert_eq!(said.len(), 1, "{warnings:?}");
        assert_eq!(said[0].code, codes::FONT_SUBSTITUTED);
        assert!(said[0].message.contains("Arial"));
    }
}
