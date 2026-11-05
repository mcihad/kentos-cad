//! `cad.standards.check` v1 and `cad.standards.apply` v1 (docs/adr/0312
//! §2–§3): what a drawing breaks of a drawing standard, and the separate,
//! planned fix. The check writes nothing; its findings come in the
//! standard's layers' order, then the drawing's other layers in the tree's,
//! then the project's font and its styles, then the objects. The apply merges
//! the layers that belong to a standard layer into it (Katmanları çevir's
//! write), adds the standard layers the drawing lacks, gives the others their
//! look and Yazdır (one undo step “Standardı uygula”), then the fonts and
//! the missing styles (project settings, docs/adr/0183). The shared cases
//! are `fixtures/commands/v1/cad.standards.{check,apply}.json`
//! (`scripts/fixtures/standards_cases.py`).

use kentos_commands::{
    CommandResult, CommandWarning, DimensionStyleDef, DrawingFont, DrawingStandard, LayerNode,
    LayerNodeType, LayerStyle, LayersTranslate, LineType, StandardConflict, StandardIssue,
    StandardIssueKind, StandardOrigin, StandardProvenance, StandardsApplied, StandardsApply,
    StandardsCheck, StandardsConflicts, StandardsConflictsReport, StandardsReport,
    StandardsResolve, StandardsResolved, TextStyleDef, TranslateRule, built_in_standard,
    built_in_standards, fold_name, font_id, font_label, pattern_matches, standard_problem,
};
use kentos_domain::{Document, NewLayer, Uuid};

use crate::checks::{self, Stop, error};
use crate::{ExecutionContext, codes, layers_translate, style_tables};

/// The step's name.
pub const APPLY: &str = "Standardı uygula";

/// `cad.standards.check`'s operations as the registry calls them.
pub mod check {
    pub use super::{execute_check as execute, plan_check as plan, validate_check as validate};
}

/// `cad.standards.apply`'s operations as the registry calls them.
pub mod apply {
    pub use super::{execute_apply as execute, plan_apply as plan, validate_apply as validate};
}

pub mod conflicts {
    pub use super::{
        execute_conflicts as execute, plan_conflicts as plan, validate_conflicts as validate,
    };
}

pub mod resolve {
    pub use super::{
        execute_resolve as execute, plan_resolve as plan, validate_resolve as validate,
    };
}

pub fn validate_resolve(cx: &ExecutionContext<'_>, _input: &StandardsResolve) -> CommandResult<()> {
    match read_project_packages(cx.doc) {
        Ok(packages) if packages.is_empty() => fail(
            codes::NO_STANDARD,
            "Projeye bağlı standart yok.".into(),
            "standardDocs",
        )
        .into(),
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan_resolve(
    cx: &ExecutionContext<'_>,
    _input: &StandardsResolve,
) -> CommandResult<StandardsResolved> {
    resolve_project(cx.doc)
}

pub fn execute_resolve(
    cx: &mut ExecutionContext<'_>,
    _input: StandardsResolve,
) -> CommandResult<StandardsResolved> {
    resolve_project(cx.doc)
}

fn resolve_project(doc: &Document) -> CommandResult<StandardsResolved> {
    use std::collections::BTreeSet;
    let packages = match read_project_packages(doc) {
        Ok(packages) if !packages.is_empty() => packages,
        Ok(_) => {
            return fail(
                codes::NO_STANDARD,
                "Projeye bağlı standart yok.".into(),
                "standardDocs",
            )
            .into();
        }
        Err(stop) => return stop.into(),
    };
    let mut result = packages[0].standard.clone();
    let origin = |package: &kentos_commands::StandardPackage| StandardOrigin {
        id: package.id.clone(),
        revision: package.revision.clone(),
    };
    let first_origin = origin(&packages[0]);
    let mut provenance = StandardProvenance {
        rules: first_origin.clone(),
        layers: result
            .layers
            .iter()
            .map(|item| (item.name.clone(), first_origin.clone()))
            .collect(),
        text_styles: result
            .text_styles
            .iter()
            .map(|item| (item.name.clone(), first_origin.clone()))
            .collect(),
        dimension_styles: result
            .dimension_styles
            .iter()
            .map(|item| (item.name.clone(), first_origin.clone()))
            .collect(),
    };
    let mut layers: BTreeSet<_> = result
        .layers
        .iter()
        .map(|item| fold_name(item.name.trim()))
        .collect();
    let mut texts: BTreeSet<_> = result
        .text_styles
        .iter()
        .map(|item| fold_name(item.name.trim()))
        .collect();
    let mut dimensions: BTreeSet<_> = result
        .dimension_styles
        .iter()
        .map(|item| fold_name(item.name.trim()))
        .collect();
    for package in packages.iter().skip(1) {
        for item in &package.standard.layers {
            if layers.insert(fold_name(item.name.trim())) {
                provenance.layers.insert(item.name.clone(), origin(package));
                result.layers.push(item.clone());
            }
        }
        for item in &package.standard.text_styles {
            if texts.insert(fold_name(item.name.trim())) {
                provenance
                    .text_styles
                    .insert(item.name.clone(), origin(package));
                result.text_styles.push(item.clone());
            }
        }
        for item in &package.standard.dimension_styles {
            if dimensions.insert(fold_name(item.name.trim())) {
                provenance
                    .dimension_styles
                    .insert(item.name.clone(), origin(package));
                result.dimension_styles.push(item.clone());
            }
        }
    }
    result.name = "Projeye bağlı standartlar".into();
    result.description = format!(
        "Genel kurallar ilk kaynaktan, her adlı tanım ilk bulunduğu kaynaktan alınır. İlk kaynak: {}.",
        packages[0].id
    );
    if let Some(why) = standard_problem(&result) {
        return fail(codes::INVALID_STANDARD, why, "standardDocs").into();
    }
    CommandResult::Completed {
        output: StandardsResolved {
            standard: result,
            sources: packages.into_iter().map(|p| p.id).collect(),
            provenance,
            revision: doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}

pub fn validate_conflicts(
    _cx: &ExecutionContext<'_>,
    _input: &StandardsConflicts,
) -> CommandResult<()> {
    CommandResult::Completed {
        output: (),
        warnings: Vec::new(),
    }
}

pub fn plan_conflicts(
    cx: &ExecutionContext<'_>,
    _input: &StandardsConflicts,
) -> CommandResult<StandardsConflictsReport> {
    project_conflicts(cx.doc)
}

pub fn execute_conflicts(
    cx: &mut ExecutionContext<'_>,
    _input: StandardsConflicts,
) -> CommandResult<StandardsConflictsReport> {
    project_conflicts(cx.doc)
}

fn read_project_packages(doc: &Document) -> Result<Vec<kentos_commands::StandardPackage>, Stop> {
    let mut packages = Vec::new();
    for (index, document) in doc.settings().standard_docs.iter().enumerate() {
        match kentos_commands::parse_standard_document(document.text.as_bytes()) {
            Ok((_, Some(package))) => packages.push(package),
            Ok((_, None)) => {
                return Err(fail(
                    codes::INVALID_STANDARD,
                    "Bağlı standart sürümlü değil.".into(),
                    &format!("standardDocs[{index}]"),
                ));
            }
            Err(why) => {
                return Err(fail(
                    codes::INVALID_STANDARD,
                    why,
                    &format!("standardDocs[{index}]"),
                ));
            }
        }
    }
    Ok(packages)
}

fn project_conflicts(doc: &Document) -> CommandResult<StandardsConflictsReport> {
    use std::collections::BTreeMap;
    fn values(fields: &[String], value: serde_json::Value) -> BTreeMap<String, serde_json::Value> {
        fields
            .iter()
            .map(|field| {
                (
                    field.clone(),
                    value.get(field).cloned().unwrap_or(serde_json::Value::Null),
                )
            })
            .collect()
    }
    fn global_rules(standard: &DrawingStandard) -> serde_json::Value {
        serde_json::json!({"naming":standard.naming, "fonts":standard.fonts,
            "lineTypes":standard.line_types, "byLayer":standard.by_layer})
    }
    fn layer_values(layer: &kentos_commands::StandardLayer) -> serde_json::Value {
        serde_json::json!({"color":layer.color, "lineType":layer.line_type,
            "lineWeight":layer.line_weight, "plot":layer.plot, "from":layer.from})
    }
    let packages = match read_project_packages(doc) {
        Ok(packages) => packages,
        Err(stop) => return stop.into(),
    };
    let mut layers: BTreeMap<String, (&String, &kentos_commands::StandardLayer)> = BTreeMap::new();
    let mut texts: BTreeMap<String, (&String, &kentos_commands::StandardTextStyle)> =
        BTreeMap::new();
    let mut dimensions: BTreeMap<String, (&String, &kentos_commands::StandardDimensionStyle)> =
        BTreeMap::new();
    let mut conflicts = Vec::new();
    for package in &packages {
        if let Some(first) = packages.first().filter(|first| first.id != package.id) {
            let mut fields = Vec::new();
            if first.standard.naming != package.standard.naming {
                fields.push("naming".into());
            }
            if first.standard.fonts != package.standard.fonts {
                fields.push("fonts".into());
            }
            if first.standard.line_types != package.standard.line_types {
                fields.push("lineTypes".into());
            }
            if first.standard.by_layer != package.standard.by_layer {
                fields.push("byLayer".into());
            }
            if !fields.is_empty() {
                let first_values = values(&fields, global_rules(&first.standard));
                let later_values = values(&fields, global_rules(&package.standard));
                conflicts.push(StandardConflict {
                    kind: "rules".into(),
                    name: "Genel kurallar".into(),
                    first_id: first.id.clone(),
                    later_id: package.id.clone(),
                    fields,
                    first_values,
                    later_values,
                });
            }
        }
        for layer in &package.standard.layers {
            let key = fold_name(layer.name.trim());
            if let Some((first_id, first)) = layers.get(&key) {
                let first: &kentos_commands::StandardLayer = first;
                let mut fields = Vec::new();
                if !first.color.eq_ignore_ascii_case(&layer.color) {
                    fields.push("color".into());
                }
                if first.line_type != layer.line_type {
                    fields.push("lineType".into());
                }
                if (first.line_weight - layer.line_weight).abs() > 1e-9 {
                    fields.push("lineWeight".into());
                }
                if first.plot != layer.plot {
                    fields.push("plot".into());
                }
                if first.from != layer.from {
                    fields.push("from".into());
                }
                if !fields.is_empty() {
                    let first_values = values(&fields, layer_values(first));
                    let later_values = values(&fields, layer_values(layer));
                    conflicts.push(StandardConflict {
                        kind: "layer".into(),
                        name: layer.name.clone(),
                        first_id: (*first_id).clone(),
                        later_id: package.id.clone(),
                        fields,
                        first_values,
                        later_values,
                    });
                }
            } else {
                layers.insert(key, (&package.id, layer));
            }
        }
        for style in &package.standard.text_styles {
            let key = fold_name(style.name.trim());
            if let Some((first_id, first)) = texts.get(&key) {
                let first: &kentos_commands::StandardTextStyle = first;
                let mut fields = Vec::new();
                if first.font != style.font {
                    fields.push("font".into());
                }
                let height_differs = match (first.height, style.height) {
                    (Some(a), Some(b)) => (a - b).abs() > 1e-9,
                    (None, None) => false,
                    _ => true,
                };
                if height_differs {
                    fields.push("height".into());
                }
                if !fields.is_empty() {
                    let first_values = values(&fields, serde_json::json!(first));
                    let later_values = values(&fields, serde_json::json!(style));
                    conflicts.push(StandardConflict {
                        kind: "textStyle".into(),
                        name: style.name.clone(),
                        first_id: (*first_id).clone(),
                        later_id: package.id.clone(),
                        fields,
                        first_values,
                        later_values,
                    });
                }
            } else {
                texts.insert(key, (&package.id, style));
            }
        }
        for style in &package.standard.dimension_styles {
            let key = fold_name(style.name.trim());
            if let Some((first_id, first)) = dimensions.get(&key) {
                let first: &kentos_commands::StandardDimensionStyle = first;
                if (first.height - style.height).abs() > 1e-9 {
                    let fields = vec!["height".into()];
                    let first_values = values(&fields, serde_json::json!(first));
                    let later_values = values(&fields, serde_json::json!(style));
                    conflicts.push(StandardConflict {
                        kind: "dimensionStyle".into(),
                        name: style.name.clone(),
                        first_id: (*first_id).clone(),
                        later_id: package.id.clone(),
                        fields,
                        first_values,
                        later_values,
                    });
                }
            } else {
                dimensions.insert(key, (&package.id, style));
            }
        }
    }
    CommandResult::Completed {
        output: StandardsConflictsReport {
            conflicts,
            revision: doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The standard an input names or gives; or why not.
pub(crate) fn standard_of(
    name: Option<&str>,
    given: Option<&DrawingStandard>,
) -> Result<DrawingStandard, Stop> {
    let standard = match (name, given) {
        (_, Some(s)) => s.clone(),
        (Some(n), None) => built_in_standard(n).ok_or_else(|| {
            fail(
                codes::STANDARD_NOT_FOUND,
                format!(
                    "“{n}” adlı yerleşik standart yok; bunlardan birini verin: {}.",
                    built_in_standards().join(", ")
                ),
                "name",
            )
        })?,
        (None, None) => {
            return Err(fail(
                codes::NO_STANDARD,
                format!(
                    "Standardı adıyla (name: {}) ya da kendisini (standard) verin.",
                    built_in_standards().join(", ")
                ),
                "name",
            ));
        }
    };
    if let Some(p) = standard_problem(&standard) {
        return Err(fail(
            codes::INVALID_STANDARD,
            format!("Standart geçersiz: {p}."),
            "standard",
        ));
    }
    Ok(standard)
}

fn leaves<'a>(nodes: &'a [LayerNode], out: &mut Vec<&'a LayerNode>) {
    for n in nodes {
        if n.kind == LayerNodeType::Layer && n.service.is_none() {
            out.push(n);
        }
        leaves(&n.children, out);
    }
}

fn weight_text(w: f64) -> String {
    format!("{w}")
}

fn plot_text(p: bool) -> &'static str {
    if p { "açık" } else { "kapalı" }
}

fn types_text(types: &[LineType]) -> String {
    types
        .iter()
        .map(|t| t.label())
        .collect::<Vec<_>>()
        .join(", ")
}

fn fonts_text(fonts: &[DrawingFont]) -> String {
    fonts
        .iter()
        .map(|f| font_label(*f))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The standard layer a drawing's layer belongs to by its `from`: its index.
fn belongs(standard: &DrawingStandard, name: &str) -> Option<usize> {
    standard
        .layers
        .iter()
        .position(|l| l.from.as_deref().is_some_and(|p| pattern_matches(p, name)))
}

fn standard_index(standard: &DrawingStandard, name: &str) -> Option<usize> {
    let folded = fold_name(name.trim());
    standard
        .layers
        .iter()
        .position(|l| fold_name(l.name.trim()) == folded)
}

/// The findings, in their order.
pub fn issues(doc: &Document, standard: &DrawingStandard) -> Vec<StandardIssue> {
    let tree = doc.layers();
    let mut all = Vec::new();
    leaves(tree.nodes(), &mut all);
    let mut out = Vec::new();
    let mut issue = |kind,
                     subject: &str,
                     layer: Option<&str>,
                     expected: Option<String>,
                     found: Option<String>,
                     objects: usize,
                     fixable: bool,
                     message: String| {
        out.push(StandardIssue {
            kind,
            subject: subject.to_owned(),
            layer: layer.map(str::to_owned),
            expected,
            found,
            objects: count(objects),
            fixable,
            message,
        });
    };
    // The standard's layers.
    for l in &standard.layers {
        let name = l.name.trim();
        let Some(n) = all.iter().find(|n| fold_name(&n.name) == fold_name(name)) else {
            issue(
                StandardIssueKind::MissingLayer,
                name,
                None,
                Some(name.to_owned()),
                None,
                0,
                true,
                format!("“{name}” katmanı çizimde yok; Standardı uygula onu ekler."),
            );
            continue;
        };
        let plot = n.plot != Some(false);
        // A named pattern takes precedence over the built-in fallback in the
        // renderer. The v1 standard describes only built-in types, so it
        // cannot decide which named definition is equivalent or replace it.
        if let Some(pattern) = n.style.line_pattern.as_deref() {
            issue(
                StandardIssueKind::LayerLineType,
                &n.name,
                Some(&n.id),
                Some(l.line_type.name().to_owned()),
                Some(pattern.to_owned()),
                0,
                false,
                format!(
                    "“{}” katmanı “{}” adlı özel çizgi tipini kullanıyor; standart {} istiyor. Çizgi tipini açıkça seçin; standart bu tanımı otomatik değiştirmez.",
                    n.name,
                    pattern,
                    l.line_type.label()
                ),
            );
        }
        let differences = [
            (
                StandardIssueKind::LayerColor,
                "rengi",
                fold_name(&l.color) == fold_name(&n.style.color),
                (l.color.clone(), n.style.color.clone()),
                (l.color.clone(), n.style.color.clone()),
            ),
            (
                StandardIssueKind::LayerLineType,
                "çizgi tipi",
                l.line_type == n.style.line_type,
                (
                    l.line_type.name().to_owned(),
                    n.style.line_type.name().to_owned(),
                ),
                (
                    l.line_type.label().to_owned(),
                    n.style.line_type.label().to_owned(),
                ),
            ),
            (
                StandardIssueKind::LayerWeight,
                "kalınlığı",
                (l.line_weight - n.style.line_weight).abs() <= 1e-9,
                (weight_text(l.line_weight), weight_text(n.style.line_weight)),
                (
                    format!("{} mm", weight_text(l.line_weight)),
                    format!("{} mm", weight_text(n.style.line_weight)),
                ),
            ),
            (
                StandardIssueKind::LayerPlot,
                "Yazdır'ı",
                l.plot == plot,
                (plot_text(l.plot).to_owned(), plot_text(plot).to_owned()),
                (plot_text(l.plot).to_owned(), plot_text(plot).to_owned()),
            ),
        ];
        for (kind, what, same, (expected, found), (want, have)) in differences {
            if !same {
                issue(
                    kind,
                    &n.name,
                    Some(&n.id),
                    Some(expected),
                    Some(found),
                    0,
                    true,
                    format!("“{}” katmanının {what} {have}, standartta {want}.", n.name),
                );
            }
        }
    }
    // The drawing's other layers.
    for n in &all {
        if standard_index(standard, &n.name).is_some() {
            continue;
        }
        if let Some(i) = belongs(standard, &n.name) {
            let to = standard.layers[i].name.trim();
            let locked = tree.is_locked(&n.id);
            let message = if locked {
                format!(
                    "“{}” katmanı standartta yok ve kilitli; kilidini açın, Standardı uygula nesnelerini “{to}” katmanına taşır.",
                    n.name
                )
            } else {
                format!(
                    "“{}” katmanı standartta yok; Standardı uygula nesnelerini “{to}” katmanına taşır.",
                    n.name
                )
            };
            issue(
                StandardIssueKind::LayerName,
                &n.name,
                Some(&n.id),
                Some(to.to_owned()),
                Some(n.name.clone()),
                0,
                !locked,
                message,
            );
        } else if !standard.naming.iter().any(|p| pattern_matches(p, &n.name)) {
            let message = if standard.naming.is_empty() {
                format!(
                    "“{}” adı standartta yok; katmanı yeniden adlandırın ya da Katmanları çevir'le bir standart katmana taşıyın.",
                    n.name
                )
            } else {
                format!(
                    "“{}” adı standartta yok ve {} adlarına uymuyor; katmanı yeniden adlandırın ya da Katmanları çevir'le bir standart katmana taşıyın.",
                    n.name,
                    standard.naming.join(", ")
                )
            };
            issue(
                StandardIssueKind::LayerName,
                &n.name,
                Some(&n.id),
                None,
                Some(n.name.clone()),
                0,
                false,
                message,
            );
        }
        if !standard.line_types.is_empty()
            && let Some(pattern) = n.style.line_pattern.as_deref()
        {
            issue(
                StandardIssueKind::LayerLineType,
                &n.name,
                Some(&n.id),
                Some(
                    standard
                        .line_types
                        .iter()
                        .map(|t| t.name())
                        .collect::<Vec<_>>()
                        .join(","),
                ),
                Some(pattern.to_owned()),
                0,
                false,
                format!(
                    "“{}” katmanı “{}” adlı özel çizgi tipini kullanıyor; standart yerleşik çizgi tipleri tarif ediyor. Eşlemeyi elle değerlendirin.",
                    n.name, pattern
                ),
            );
        }
        if !standard.line_types.is_empty() && !standard.line_types.contains(&n.style.line_type) {
            issue(
                StandardIssueKind::LayerLineType,
                &n.name,
                Some(&n.id),
                Some(
                    standard
                        .line_types
                        .iter()
                        .map(|t| t.name())
                        .collect::<Vec<_>>()
                        .join(","),
                ),
                Some(n.style.line_type.name().to_owned()),
                0,
                false,
                format!(
                    "“{}” katmanının çizgi tipi {}; standart yalnız {} çizgi tiplerini kullanır.",
                    n.name,
                    n.style.line_type.label(),
                    types_text(&standard.line_types)
                ),
            );
        }
    }
    // The fonts and the styles.
    let settings = doc.settings();
    if let Some(&first) = standard.fonts.first() {
        let font = settings.drawing_font.unwrap_or(DrawingFont::Barlow);
        if !standard.fonts.contains(&font) {
            issue(
                StandardIssueKind::ProjectFont,
                font_label(font),
                None,
                Some(font_id(first).to_owned()),
                Some(font_id(font).to_owned()),
                0,
                true,
                format!(
                    "Projenin yazı tipi {}; standart {} kullanır.",
                    font_label(font),
                    fonts_text(&standard.fonts)
                ),
            );
        }
    }
    for s in &settings.text_styles {
        let wanted = standard
            .text_styles
            .iter()
            .find(|t| fold_name(t.name.trim()) == fold_name(s.name.trim()));
        let expected = match wanted {
            Some(t) => Some(t.font),
            None => standard
                .fonts
                .first()
                .copied()
                .filter(|_| !standard.fonts.contains(&s.font)),
        };
        if let Some(f) = expected.filter(|f| *f != s.font) {
            issue(
                StandardIssueKind::TextFont,
                &s.name,
                None,
                Some(font_id(f).to_owned()),
                Some(font_id(s.font).to_owned()),
                0,
                true,
                format!(
                    "“{}” yazı stilinin yazı tipi {}, standartta {}.",
                    s.name,
                    font_label(s.font),
                    font_label(f)
                ),
            );
        }
        if let Some(height) = wanted.and_then(|t| t.height)
            && s.height.is_none_or(|found| (found - height).abs() > 1e-9)
        {
            let found = s
                .height
                .map(|h| format!("{h} mm"))
                .unwrap_or_else(|| "araç varsayılanı".to_owned());
            issue(
                StandardIssueKind::TextStyleHeight,
                &s.name,
                None,
                Some(format!("{height} mm")),
                Some(found.clone()),
                0,
                false,
                format!(
                    "“{}” yazı stilinin kâğıt yüksekliği {found}; standartta {height} mm. Stili elle düzenleyin.",
                    s.name
                ),
            );
        }
        if wanted.is_none() && !standard.text_styles.is_empty() {
            issue(
                StandardIssueKind::TextStyleName,
                &s.name,
                None,
                None,
                Some(s.name.clone()),
                0,
                false,
                format!(
                    "“{}” yazı stili standartta yok; yazılarını standardın bir stiline geçirin.",
                    s.name
                ),
            );
        }
    }
    for t in &standard.text_styles {
        let name = t.name.trim();
        if !settings
            .text_styles
            .iter()
            .any(|s| fold_name(s.name.trim()) == fold_name(name))
        {
            issue(
                StandardIssueKind::MissingTextStyle,
                name,
                None,
                Some(name.to_owned()),
                None,
                0,
                true,
                format!("“{name}” yazı stili projede yok; Standardı uygula onu ekler."),
            );
        }
    }
    for s in &settings.dimension_styles {
        if let Some(wanted) = standard
            .dimension_styles
            .iter()
            .find(|d| fold_name(d.name.trim()) == fold_name(s.name.trim()))
            && (s.height - wanted.height).abs() > 1e-9
        {
            issue(
                StandardIssueKind::DimensionStyleHeight,
                &s.name,
                None,
                Some(format!("{} mm", wanted.height)),
                Some(format!("{} mm", s.height)),
                0,
                false,
                format!(
                    "“{}” ölçü stilinin kâğıt yazı yüksekliği {} mm; standartta {} mm. Stili elle düzenleyin.",
                    s.name, s.height, wanted.height
                ),
            );
        }
        if !standard.dimension_styles.is_empty()
            && !standard
                .dimension_styles
                .iter()
                .any(|d| fold_name(d.name.trim()) == fold_name(s.name.trim()))
        {
            issue(
                StandardIssueKind::DimensionStyleName,
                &s.name,
                None,
                None,
                Some(s.name.clone()),
                0,
                false,
                format!(
                    "“{}” ölçü stili standartta yok; ölçülerini standardın bir stiline geçirin.",
                    s.name
                ),
            );
        }
    }
    for d in &standard.dimension_styles {
        let name = d.name.trim();
        if !settings
            .dimension_styles
            .iter()
            .any(|s| fold_name(s.name.trim()) == fold_name(name))
        {
            issue(
                StandardIssueKind::MissingDimensionStyle,
                name,
                None,
                Some(name.to_owned()),
                None,
                0,
                true,
                format!("“{name}” ölçü stili projede yok; Standardı uygula onu ekler."),
            );
        }
    }
    // The objects.
    let mut by_type = [0usize; 4];
    let mut named_types = std::collections::BTreeMap::<String, (String, usize)>::new();
    let mut own = 0usize;
    for e in doc.entities() {
        let b = e.base();
        if let Some(t) = b.look.line_type {
            by_type[LineType::ALL.iter().position(|x| *x == t).unwrap_or(0)] += 1;
        }
        if let Some(pattern) = b.look.line_pattern.as_deref() {
            let entry = named_types
                .entry(fold_name(pattern))
                .or_insert_with(|| (pattern.to_owned(), 0));
            entry.1 += 1;
        }
        if b.color.is_some()
            || b.line_weight.is_some()
            || b.look.line_type.is_some()
            || b.look.line_pattern.is_some()
        {
            own += 1;
        }
    }
    if !standard.line_types.is_empty() {
        // Definitions are checked once, not once per placement. A named
        // pattern inside a reusable block is not necessarily present on any
        // top-level object's base, including when the block is unused.
        for block in doc.blocks() {
            let mut patterns = std::collections::BTreeMap::<String, (String, usize)>::new();
            let mut types = [0usize; 4];
            for part in &block.entities {
                if let Some(name) = part.base().look.line_pattern.as_deref() {
                    let entry = patterns
                        .entry(fold_name(name))
                        .or_insert_with(|| (name.to_owned(), 0));
                    entry.1 += 1;
                } else if let Some(line_type) = part.base().look.line_type
                    && let Some(index) = LineType::ALL.iter().position(|t| *t == line_type)
                {
                    types[index] += 1;
                }
            }
            for (_, (name, pieces)) in patterns {
                issue(
                    StandardIssueKind::ObjectLineType,
                    &format!("Blok: {}", block.name),
                    None,
                    Some(types_text(&standard.line_types)),
                    Some(name.clone()),
                    0,
                    false,
                    format!(
                        "“{}” blok tanımındaki {pieces} parça “{name}” özel çizgi tipini kullanıyor; standart yalnız {} yerleşik çizgi tiplerini tanımlar. Tanım bir kez denetlendi; yerleştirme sayısı bu bulguya dahil değildir.",
                        block.name,
                        types_text(&standard.line_types)
                    ),
                );
            }
            for (index, line_type) in LineType::ALL.into_iter().enumerate() {
                let pieces = types[index];
                if pieces > 0 && !standard.line_types.contains(&line_type) {
                    issue(
                        StandardIssueKind::ObjectLineType,
                        &format!("Blok: {}", block.name),
                        None,
                        Some(types_text(&standard.line_types)),
                        Some(line_type.name().to_owned()),
                        0,
                        false,
                        format!(
                            "“{}” blok tanımındaki {pieces} parça {} çizgi tipini kullanıyor; standart yalnız {} çizgi tiplerini tanımlar. Tanım bir kez denetlendi; yerleştirme sayısı bu bulguya dahil değildir.",
                            block.name,
                            line_type.label(),
                            types_text(&standard.line_types)
                        ),
                    );
                }
            }
        }
        for (_, (name, objects)) in named_types {
            issue(
                StandardIssueKind::ObjectLineType,
                &name,
                None,
                Some(types_text(&standard.line_types)),
                Some(name.clone()),
                objects,
                false,
                format!(
                    "{objects} nesnenin kendi özel çizgi tipi “{name}”; standart yalnız {} yerleşik çizgi tiplerini tanımlar. Eşleme bilinmediği için çizgi tipini elle seçin.",
                    types_text(&standard.line_types)
                ),
            );
        }
        for (i, t) in LineType::ALL.into_iter().enumerate() {
            if by_type[i] > 0 && !standard.line_types.contains(&t) {
                issue(
                    StandardIssueKind::ObjectLineType,
                    t.label(),
                    None,
                    None,
                    Some(t.name().to_owned()),
                    by_type[i],
                    false,
                    format!(
                        "{} nesnenin kendi çizgi tipi {}; standart yalnız {} çizgi tiplerini kullanır.",
                        by_type[i],
                        t.label(),
                        types_text(&standard.line_types)
                    ),
                );
            }
        }
    }
    if standard.by_layer && own > 0 {
        issue(
            StandardIssueKind::OwnLook,
            "Katmana göre",
            None,
            None,
            None,
            own,
            false,
            format!(
                "{own} nesnenin kendi rengi, çizgi tipi ya da kalınlığı var; standartta nesneler görünüşünü katmanından alır. Özellik kopyala ya da Öznitelikler'in “Katmana göre”siyle düzeltin."
            ),
        );
    }
    if standard.by_layer {
        for block in doc.blocks() {
            let pieces = block
                .entities
                .iter()
                .filter(|part| {
                    let b = part.base();
                    b.color.is_some()
                        || b.line_weight.is_some()
                        || b.look.line_type.is_some()
                        || b.look.line_pattern.is_some()
                })
                .count();
            if pieces > 0 {
                issue(
                    StandardIssueKind::OwnLook,
                    &format!("Blok: {}", block.name),
                    None,
                    Some("Katmana göre".to_owned()),
                    Some(format!("{pieces} parça")),
                    0,
                    false,
                    format!(
                        "“{}” blok tanımındaki {pieces} parçada açıkça atanmış renk, çizgi tipi veya kalınlık var; standart katmana göre görünüş ister. Tanım bir kez denetlendi; yerleştirme sayısı bu bulguya dahil değildir.",
                        block.name
                    ),
                );
            }
        }
    }
    out
}

// ── cad.standards.check ─────────────────────────────────────────────────────

/// Checks `input`.
pub fn validate_check(_cx: &ExecutionContext<'_>, input: &StandardsCheck) -> CommandResult<()> {
    match standard_of(input.name.as_deref(), input.standard.as_ref()) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The report; the same as [`execute_check`].
pub fn plan_check(
    cx: &ExecutionContext<'_>,
    input: &StandardsCheck,
) -> CommandResult<StandardsReport> {
    report(cx.doc, input)
}

/// The report. Writes nothing.
pub fn execute_check(
    cx: &mut ExecutionContext<'_>,
    input: StandardsCheck,
) -> CommandResult<StandardsReport> {
    report(cx.doc, &input)
}

fn report(doc: &Document, input: &StandardsCheck) -> CommandResult<StandardsReport> {
    let standard = match standard_of(input.name.as_deref(), input.standard.as_ref()) {
        Ok(s) => s,
        Err(stop) => return stop.into(),
    };
    let issues = issues(doc, &standard);
    CommandResult::Completed {
        output: StandardsReport {
            standard: standard.name.clone(),
            fixable: count(issues.iter().filter(|i| i.fixable).count()),
            issues,
            revision: doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}

// ── cad.standards.apply ─────────────────────────────────────────────────────

fn check_apply(doc: &Document, input: &StandardsApply) -> Result<DrawingStandard, Stop> {
    let standard = standard_of(input.name.as_deref(), input.standard.as_ref())?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    if let Some(parent) = input.parent.as_deref() {
        match doc.layers().get(parent) {
            None => {
                return Err(fail(
                    codes::LAYER_NOT_FOUND,
                    format!(
                        "“{parent}” kimlikli katman ya da grup çizimde yok. Var olan bir grubun kimliğini verin."
                    ),
                    "parent",
                ));
            }
            Some(g) if g.kind != LayerNodeType::Group => {
                return Err(fail(
                    codes::NOT_A_GROUP,
                    format!(
                        "“{}” bir katman; yeni katmanlar yalnız bir grubun içine eklenir. Bir grubun kimliğini verin ya da parent'ı vermeyin (en üst).",
                        g.name
                    ),
                    "parent",
                ));
            }
            _ => {}
        }
    }
    if input.rename
        && let Some(rules) = rules_of(doc, &standard)
    {
        // Katmanları çevir's refusals, at the standard: its rules are the standard's layers.
        layers_translate::check(doc, &translate_input(rules, input.parent.clone())).map_err(
            |stop| {
                let mut e = stop.into_error();
                if e.path.as_deref().is_some_and(|p| p.starts_with("rules")) {
                    e.path = Some("standard".into());
                }
                Stop::failed(e)
            },
        )?;
    }
    Ok(standard)
}

/// The translation's rules: one per standard layer with layers to merge; none when nothing belongs.
fn rules_of(doc: &Document, standard: &DrawingStandard) -> Option<Vec<TranslateRule>> {
    let mut all = Vec::new();
    leaves(doc.layers().nodes(), &mut all);
    let any = all.iter().any(|n| {
        standard_index(standard, &n.name).is_none() && belongs(standard, &n.name).is_some()
    });
    any.then(|| {
        standard
            .layers
            .iter()
            .filter_map(|l| {
                Some(TranslateRule {
                    from: l.from.clone()?,
                    to: l.name.trim().to_owned(),
                    color: Some(l.color.clone()),
                    line_type: Some(l.line_type),
                    line_weight: Some(l.line_weight),
                })
            })
            .collect()
    })
}

fn translate_input(rules: Vec<TranslateRule>, parent: Option<String>) -> LayersTranslate {
    LayersTranslate {
        rules,
        parent,
        expected_revision: None,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate_apply(cx: &ExecutionContext<'_>, input: &StandardsApply) -> CommandResult<()> {
    match check_apply(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The preview: the same write on a reading copy; the drawing stays.
pub fn plan_apply(
    cx: &ExecutionContext<'_>,
    input: &StandardsApply,
) -> CommandResult<StandardsApplied> {
    let standard = match check_apply(cx.doc, input) {
        Ok(s) => s,
        Err(stop) => return stop.into(),
    };
    let mut copy = cx.doc.reading_copy();
    match write(&mut copy, input, &standard, &|_| "-".to_owned()) {
        CommandResult::Completed {
            mut output,
            warnings,
        } => {
            output.revision = cx.doc.revision().to_string();
            CommandResult::Completed { output, warnings }
        }
        other => other,
    }
}

/// Brings the drawing to the standard.
pub fn execute_apply(
    cx: &mut ExecutionContext<'_>,
    input: StandardsApply,
) -> CommandResult<StandardsApplied> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_STANDARDS_APPLY,
        kentos_commands::CAD_STANDARDS_APPLY_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: StandardsApply,
) -> CommandResult<StandardsApplied> {
    let standard = match check_apply(cx.doc, &input) {
        Ok(s) => s,
        Err(stop) => return stop.into(),
    };
    write(cx.doc, &input, &standard, &|_| Uuid::now_v7().to_string())
}

pub(crate) fn write(
    doc: &mut Document,
    input: &StandardsApply,
    standard: &DrawingStandard,
    fresh: &dyn Fn(usize) -> String,
) -> CommandResult<StandardsApplied> {
    let mut out = StandardsApplied {
        changed: false,
        created: Vec::new(),
        restyled: Vec::new(),
        translations: Vec::new(),
        objects: 0,
        text_styles: Vec::new(),
        dimension_styles: Vec::new(),
        font: false,
        revision: doc.revision().to_string(),
    };
    let mut warnings: Vec<CommandWarning> = Vec::new();
    let group = doc.begin_group(APPLY);
    // 1. The layers that belong to a standard layer merged into it.
    if input.rename
        && let Some(rules) = rules_of(doc, standard)
    {
        match layers_translate::write(doc, &translate_input(rules, input.parent.clone())) {
            CommandResult::Completed {
                output,
                warnings: w,
            } => {
                out.translations = output.translations;
                out.objects = output.objects;
                out.created.extend(output.created);
                out.restyled.extend(output.restyled);
                warnings.extend(w);
            }
            CommandResult::Failed { error } => {
                doc.cancel_group(group);
                return CommandResult::Failed { error };
            }
            CommandResult::Conflict { error } => {
                doc.cancel_group(group);
                return CommandResult::Conflict { error };
            }
            CommandResult::NeedsInput { error } => {
                doc.cancel_group(group);
                return CommandResult::NeedsInput { error };
            }
            _ => {}
        }
    }
    // 2. The standard layers: added, or given their look and Yazdır.
    let mut created_here: Vec<(usize, String)> = Vec::new();
    for (i, l) in standard.layers.iter().enumerate() {
        let name = l.name.trim();
        let found = {
            let mut all = Vec::new();
            leaves(doc.layers().nodes(), &mut all);
            all.iter()
                .find(|n| fold_name(&n.name) == fold_name(name))
                .map(|n| (n.id.clone(), n.style.clone(), n.plot != Some(false)))
        };
        match found {
            Some((id, style, plot)) => {
                let just_made = out.created.contains(&id);
                if !(input.restyle || just_made) {
                    continue;
                }
                let next = LayerStyle {
                    color: l.color.clone(),
                    line_type: l.line_type,
                    line_weight: l.line_weight,
                    ..style.clone()
                };
                let mut changed = false;
                if next != style {
                    changed |= doc.set_layer_style(&id, next, APPLY);
                }
                if plot != l.plot {
                    changed |= doc.set_layer_plot(&id, l.plot);
                }
                if changed && !just_made && !out.restyled.contains(&id) {
                    out.restyled.push(id);
                }
            }
            None if input.layers => {
                let mut new = NewLayer::layer(name);
                new.style = LayerStyle {
                    color: l.color.clone(),
                    line_type: l.line_type,
                    line_weight: l.line_weight,
                    ..kentos_domain::default_style()
                };
                // The addition must carry its print flag too: changing this after recording
                // LayerAdd loses the non-printing value when the addition is redone.
                new.plot = (!l.plot).then_some(false);
                match doc.add_layer_at(new, input.parent.as_deref(), None, None, false) {
                    Ok(id) => {
                        created_here.push((i, id));
                    }
                    Err(r) => {
                        doc.cancel_group(group);
                        return Stop::failed(error(
                            codes::LAYER_REFUSED,
                            r.0,
                            Some("standard".into()),
                        ))
                        .into();
                    }
                }
            }
            None => {}
        }
    }
    // The added layers in the standard's order, after those Katmanları çevir added.
    created_here.sort_by_key(|(i, _)| *i);
    out.created
        .extend(created_here.into_iter().map(|(_, id)| id));
    doc.end_group(group);
    // 3. The fonts and the styles: project settings.
    if input.styles {
        let mut settings = doc.settings().clone();
        if let Some(&first) = standard.fonts.first()
            && !standard
                .fonts
                .contains(&settings.drawing_font.unwrap_or(DrawingFont::Barlow))
        {
            settings.drawing_font = Some(first);
            doc.set_settings(settings.clone());
            out.font = true;
        }
        let mut texts = doc.settings().text_styles.clone();
        let mut fresh_at = 0usize;
        let mut touched = false;
        for s in &mut texts {
            let wanted = standard
                .text_styles
                .iter()
                .find(|t| fold_name(t.name.trim()) == fold_name(s.name.trim()))
                .map(|t| t.font)
                .or_else(|| {
                    standard
                        .fonts
                        .first()
                        .copied()
                        .filter(|_| !standard.fonts.contains(&s.font))
                });
            if let Some(f) = wanted.filter(|f| *f != s.font) {
                s.font = f;
                out.text_styles.push(s.name.clone());
                touched = true;
            }
        }
        for t in &standard.text_styles {
            let name = t.name.trim();
            if !texts
                .iter()
                .any(|s| fold_name(s.name.trim()) == fold_name(name))
            {
                texts.push(TextStyleDef {
                    id: fresh(fresh_at),
                    name: name.to_owned(),
                    font: t.font,
                    bold: false,
                    italic: false,
                    oblique: None,
                    height: t.height,
                    width_factor: None,
                    font_file: None,
                });
                fresh_at += 1;
                out.text_styles.push(name.to_owned());
                touched = true;
            }
        }
        if touched {
            // The texts that follow a refonted style follow it, those on locked layers left (docs/adr/0183).
            style_tables::save_text_styles(doc, texts);
        }
        let mut dims = doc.settings().dimension_styles.clone();
        let mut added = false;
        for d in &standard.dimension_styles {
            let name = d.name.trim();
            if !dims
                .iter()
                .any(|s| fold_name(s.name.trim()) == fold_name(name))
            {
                dims.push(DimensionStyleDef {
                    id: fresh(fresh_at),
                    name: name.to_owned(),
                    height: d.height,
                    ..DimensionStyleDef::default()
                });
                fresh_at += 1;
                out.dimension_styles.push(name.to_owned());
                added = true;
            }
        }
        if added {
            style_tables::save_dimension_styles(doc, dims);
        }
    }
    out.changed = !out.created.is_empty()
        || !out.restyled.is_empty()
        || !out.translations.is_empty()
        || !out.text_styles.is_empty()
        || !out.dimension_styles.is_empty()
        || out.font;
    out.revision = doc.revision().to_string();
    CommandResult::Completed {
        output: out,
        warnings,
    }
}
