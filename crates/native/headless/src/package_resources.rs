//! Paketle's resource report (docs/adr/1111; plan/cad-advanced.md CAD-ADV-14, AutoCAD eTransmit's report of fonts,
//! shape files and plot style tables): every typeface, SHX shape file, line type, hatch pattern and plot style the
//! drawing and its external references draw with, each with how the package has it — built into KentOS, embedded in
//! the project file, carried only in a reference's source copied under `veri/`, drawn in a substitute face, or missing.
//! A missing or only-carried one is a `package_resources` warning: nothing is dropped without being said.
//!
//! Read only, from the drawing (its layers, objects and blocks, a reference's pieces among them, its settings, library
//! and sheet book) and from the references' sources as the package finds them (`xrefs::found_source_in`); a source is
//! read once. The independent reference is `scripts/fixtures/package_resource_cases.py`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use kentos_commands::{
    CommandWarning, DocumentSnapshotV2, Entity, FontMatch, FontsReport, HatchPatternType, LayerNode,
    LayerNodeType, PackageResource, PackageResourceHow as How, PackageResourceKind as Kind, codes,
    shape_resource,
};
use kentos_domain::Document;

/// A row being gathered: its users and objects added as they are met.
struct Row {
    how: How,
    file: Option<String>,
    users: BTreeSet<String>,
    objects: u32,
    note: Option<String>,
}

#[derive(Default)]
struct Rows(BTreeMap<(Kind, String), Row>);

impl Rows {
    fn row(&mut self, kind: Kind, name: &str, how: How) -> &mut Row {
        self.0.entry((kind, name.to_owned())).or_insert_with(|| Row {
            how,
            file: None,
            users: BTreeSet::new(),
            objects: 0,
            note: None,
        })
    }

    fn into_list(self) -> Vec<PackageResource> {
        self.0
            .into_iter()
            .map(|((kind, name), r)| PackageResource {
                kind,
                name,
                how: r.how,
                file: r.file,
                users: r.users.into_iter().collect(),
                objects: r.objects,
                note: r.note,
            })
            .collect()
    }
}

fn fold(s: &str) -> String {
    s.trim().to_lowercase()
}

/// Each layer (not group) of a tree with its path (`Dış başvuru: Plan / Gaz`), in the tree's order.
fn layers_of(nodes: &[LayerNode]) -> Vec<(String, &LayerNode)> {
    fn walk<'a>(nodes: &'a [LayerNode], here: &str, out: &mut Vec<(String, &'a LayerNode)>) {
        for n in nodes {
            let path = if here.is_empty() {
                n.name.clone()
            } else {
                format!("{here} / {}", n.name)
            };
            if n.kind == LayerNodeType::Layer {
                out.push((path.clone(), n));
            }
            walk(&n.children, &path, out);
        }
    }
    let mut out = Vec::new();
    walk(nodes, "", &mut out);
    out
}

/// What a reference's source defines (by folded name): its line types, hatch patterns and text styles; its text
/// styles its own objects use.
#[derive(Default)]
struct Defined {
    line_types: BTreeSet<String>,
    patterns: BTreeSet<String>,
    used_styles: Vec<(String, String)>,
}

fn defined(name: &str, d: &DocumentSnapshotV2) -> Defined {
    let mut out = Defined {
        line_types: d.settings.line_types.iter().map(|t| fold(&t.name)).collect(),
        patterns: d.settings.hatch_patterns.iter().map(|p| fold(&p.name)).collect(),
        used_styles: Vec::new(),
    };
    let blocks = d.blocks.iter().flat_map(|b| b.entities.iter());
    for e in d.entities.iter().chain(blocks) {
        if let Entity::Text(t) = e
            && let Some(id) = &t.face.text_style
            && let Some(style) = d.settings.text_styles.iter().find(|s| &s.id == id)
            && !out.used_styles.iter().any(|(_, s)| fold(s) == fold(&style.name))
        {
            out.used_styles.push((name.to_owned(), style.name.clone()));
        }
    }
    out
}

/// A line type's name as a row names it: the pattern's, else the built-in's contract name.
fn line_type_name(
    pattern: Option<&String>,
    kind: Option<kentos_commands::LineType>,
) -> Option<String> {
    pattern.cloned().or_else(|| kind.map(|k| k.name().to_owned()))
}

/// The report, and its warning when something is missing or only carried. `sources`: each reference the package
/// copies, by name, and its source as found.
pub(crate) fn report(
    doc: &Document,
    sources: &[(String, PathBuf)],
) -> (Vec<PackageResource>, Option<CommandWarning>) {
    let mut rows = Rows::default();
    let settings = doc.settings();
    let theirs: Vec<(String, Defined)> = sources
        .iter()
        .filter_map(|(name, path)| {
            kentos_native_application::xrefs::source_drawing(path).map(|d| (name.clone(), defined(name, &d)))
        })
        .collect();
    // Typefaces: each text style's, as the font report resolves it (docs/adr/0324 §5).
    let (fonts, _) = kentos_native_application::fonts_report::report(doc, &FontsReport::default());
    for f in &fonts.styles {
        let substituted = matches!(f.how, FontMatch::Shx | FontMatch::Missing);
        // `style` is the style's id, `name` its name; the file it named, else KentOS's face it is drawn in.
        let r = rows.row(
            Kind::Font,
            &f.name,
            if substituted { How::Substituted } else { How::BuiltIn },
        );
        r.file = Some(f.file.clone().unwrap_or_else(|| f.face.id().to_owned()));
        r.users.insert(format!("stil:{}", f.name));
        r.objects += f.texts;
        if substituted {
            r.note = Some("KentOS'ta bu yazı tipi yok; yazıları yerine başka yüzle çizilir".into());
        }
    }
    // A reference's own text style the drawing has none of: its texts came in drawn in the drawing's default.
    let ours: BTreeSet<String> = settings.text_styles.iter().map(|s| fold(&s.name)).collect();
    for (_, d) in &theirs {
        for (xref, style) in &d.used_styles {
            if ours.contains(&fold(style)) || fold(style) == fold(kentos_commands::STANDARD_STYLE) {
                continue;
            }
            let r = rows.row(Kind::Font, style, How::Carried);
            r.users.insert(format!("xref:{xref}"));
            r.note = Some(
                "dış başvurunun kaynağındaki yazı stili; bu çizimde yok, yazıları Standart ile çizilir".into(),
            );
        }
    }
    // Line types: the project's own, and every one a layer or an object (the drawing's, its blocks', its references'
    // pieces) names.
    let project: BTreeMap<String, &kentos_commands::LineTypeDef> =
        settings.line_types.iter().map(|t| (fold(&t.name), t)).collect();
    for t in &settings.line_types {
        rows.row(Kind::LineType, &t.name, How::Embedded);
    }
    let line_type = |rows: &mut Rows, name: String, user: Option<String>| {
        let how = if project.contains_key(&fold(&name)) {
            How::Embedded
        } else if kentos_commands::LineType::from_name(&name).is_some() {
            How::BuiltIn
        } else if theirs.iter().any(|(_, d)| d.line_types.contains(&fold(&name))) {
            How::Carried
        } else {
            How::Missing
        };
        let key = project.get(&fold(&name)).map_or(name, |t| t.name.clone());
        let r = rows.row(Kind::LineType, &key, how);
        match user {
            Some(u) => {
                r.users.insert(u);
            }
            None => r.objects += 1,
        }
        if r.note.is_none() {
            r.note = match how {
                How::Carried => Some(
                    "dış başvurunun kaynağında tanımlı; bu çizimde yok, en yakın yerleşik tiple çizilir".into(),
                ),
                How::Missing => Some("hiçbir yerde tanımlı değil; en yakın yerleşik tiple çizilir".into()),
                _ => None,
            };
        }
    };
    for (path, n) in layers_of(doc.layers().nodes()) {
        if let Some(name) = line_type_name(n.style.line_pattern.as_ref(), Some(n.style.line_type)) {
            line_type(&mut rows, name, Some(format!("katman:{path}")));
        }
    }
    let pieces: Vec<&Entity> = doc
        .entities()
        .chain(doc.blocks().iter().flat_map(|b| b.entities.iter()))
        .collect();
    for e in &pieces {
        let look = &e.base().look;
        if let Some(name) = line_type_name(look.line_pattern.as_ref(), look.line_type) {
            line_type(&mut rows, name, None);
        }
    }
    // SHX shape files a line type's pattern places shapes of (docs/adr/0930): embedded in the library or missing.
    for t in &settings.line_types {
        for shape in t.elements.iter().filter_map(|e| e.shape.as_ref()) {
            let file = shape_resource::source_name(&shape.file);
            let id = shape_resource::id(file);
            let have = doc
                .styles()
                .items
                .iter()
                .any(|i| shape_resource::is_resource(i) && i["id"].as_str() == Some(id.as_str()));
            let r = rows.row(
                Kind::ShapeFile,
                &file.to_lowercase(),
                if have { How::Embedded } else { How::Missing },
            );
            r.file = Some(file.to_owned());
            r.users.insert(format!("çizgi tipi:{}", t.name));
            if !have {
                r.note = Some("projenin kitaplığında yok; çizgi tipinin şekilleri çizilemez".into());
            }
        }
    }
    // Hatch patterns: the project's own, and each hatch's.
    let patterns: BTreeSet<String> = settings.hatch_patterns.iter().map(|p| fold(&p.name)).collect();
    for p in &settings.hatch_patterns {
        rows.row(Kind::HatchPattern, &p.name, How::Embedded);
    }
    for e in &pieces {
        let Entity::Hatch(h) = e else {
            continue;
        };
        let p = &h.pattern;
        let (name, how, note) = match p.kind {
            HatchPatternType::Solid => ("solid".to_owned(), How::BuiltIn, None),
            HatchPatternType::Lines => ("lines".to_owned(), How::BuiltIn, None),
            HatchPatternType::Cross => ("cross".to_owned(), How::BuiltIn, None),
            HatchPatternType::Gradient => ("gradient".to_owned(), How::BuiltIn, None),
            HatchPatternType::Pattern => {
                let name = p.name.clone().unwrap_or_else(|| "adsız".to_owned());
                if patterns.contains(&fold(&name)) {
                    (name, How::Embedded, None)
                } else if p.lines.as_ref().is_some_and(|l| !l.is_empty()) {
                    (name, How::Embedded, Some("desenin çizgileri taramanın kendisinde".to_owned()))
                } else if theirs.iter().any(|(_, d)| d.patterns.contains(&fold(&name))) {
                    (
                        name,
                        How::Carried,
                        Some("dış başvurunun kaynağında tanımlı; taramada çizgisi yok".to_owned()),
                    )
                } else {
                    (name, How::Missing, Some("tanımı yok; tarama deseni çizilemez".to_owned()))
                }
            }
        };
        let key = settings
            .hatch_patterns
            .iter()
            .find(|d| fold(&d.name) == fold(&name))
            .map_or(name, |d| d.name.clone());
        let r = rows.row(Kind::HatchPattern, &key, how);
        r.objects += 1;
        if r.note.is_none() {
            r.note = note;
        }
    }
    // Plot styles: each plot profile of the book and each sheet's. The sheet core's book check keeps a profile's named
    // styles whole (an assigned or default style is among its entries, docs/adr/0609), so a profile is the unit here; a
    // book it cannot read is said, never skipped.
    let book = doc.sheets().map(kentos_sheet::project::read_project_sheets);
    if let Some(Err(e)) = &book {
        let r = rows.row(Kind::PlotStyle, "pafta kitabı", How::Missing);
        r.note = Some(format!("pafta kitabı okunamadı ({}); baskı profilleri raporlanamadı", e.message));
    }
    if let Some(Ok((book, _))) = book {
        for p in &book.plot_profiles {
            rows.row(Kind::PlotStyle, &p.name, How::Embedded);
        }
        for sheet in &book.sheets {
            let Some(id) = sheet.plot.as_ref().and_then(|p| p.settings.profile.as_ref()) else {
                continue;
            };
            let profile = sheet
                .plot
                .as_ref()
                .and_then(|p| p.settings.profile_snapshot.as_ref())
                .filter(|p| &p.id == id)
                .or_else(|| book.plot_profiles.iter().find(|p| &p.id == id));
            match profile {
                Some(p) => {
                    rows.row(Kind::PlotStyle, &p.name, How::Embedded)
                        .users
                        .insert(format!("pafta:{}", sheet.name));
                }
                None => {
                    let r = rows.row(Kind::PlotStyle, id, How::Missing);
                    r.users.insert(format!("pafta:{}", sheet.name));
                    r.note = Some("paftanın baskı profili kitapta yok".into());
                }
            }
        }
    }
    let list = rows.into_list();
    let findings: Vec<String> = list
        .iter()
        .filter(|r| matches!(r.how, How::Missing | How::Carried))
        .map(|r| {
            format!(
                "{} ({}, {})",
                r.name,
                match r.kind {
                    Kind::Font => "yazı stili",
                    Kind::ShapeFile => "SHX şekil dosyası",
                    Kind::LineType => "çizgi tipi",
                    Kind::HatchPattern => "tarama deseni",
                    Kind::PlotStyle => "baskı stili",
                },
                if r.how == How::Missing {
                    "eksik"
                } else {
                    "yalnız dış başvurunun kaynağında"
                }
            )
        })
        .collect();
    let warning = (!findings.is_empty()).then(|| CommandWarning {
        code: codes::PACKAGE_RESOURCES.into(),
        message: format!(
            "{} kaynak pakette tanımlı değil ya da yalnız dış başvurunun kaynağında: {}. Projeye ekleyip yeniden paketleyin.",
            findings.len(),
            findings.join(", ")
        ),
        path: None,
    });
    (list, warning)
}
