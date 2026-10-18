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

use kentos_contracts::{
    CommandResult, CommandWarning, DimensionStyleDef, DrawingFont, DrawingStandard, LayerNode,
    LayerNodeType, LayerStyle, LayersTranslate, LineType, StandardIssue, StandardIssueKind,
    StandardsApplied, StandardsApply, StandardsCheck, StandardsReport, TextStyleDef, TranslateRule,
    built_in_standard, built_in_standards, fold_name, font_id, font_label, pattern_matches,
    standard_problem,
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
    let mut own = 0usize;
    for e in doc.entities() {
        let b = e.base();
        if let Some(t) = b.look.line_type {
            by_type[LineType::ALL.iter().position(|x| *x == t).unwrap_or(0)] += 1;
        }
        if b.color.is_some() || b.line_weight.is_some() || b.look.line_type.is_some() {
            own += 1;
        }
    }
    if !standard.line_types.is_empty() {
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
        kentos_contracts::CAD_STANDARDS_APPLY,
        kentos_contracts::CAD_STANDARDS_APPLY_VERSION,
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
                match doc.add_layer_at(new, input.parent.as_deref(), None, None, false) {
                    Ok(id) => {
                        if !l.plot {
                            doc.set_layer_plot(&id, false);
                        }
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
