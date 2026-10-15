//! `cad.layers.translate` v1 (docs/adr/0311 §5): layers merged into a
//! drawing standard's by rules, AutoCAD's LAYTRANS. Each layer holding
//! objects takes the first rule whose pattern matches its name; its objects
//! go to the rule's standard layer (added when the drawing lacks it), it is
//! removed; all of it one undo step “Katmanları çevir”. The plan is the
//! preview the Katmanları çevir window shows. The shared cases are
//! `fixtures/commands/v1/cad.layers.translate.json`
//! (`scripts/fixtures/layer_find_cases.py`).

use std::collections::HashMap;

use kentos_contracts::{
    CommandResult, CommandWarning, EntitiesSetProperties, LayerNode, LayerNodeType,
    LayerTranslation, LayersTranslate, LayersTranslated, PropertiesOperation, TranslateRule,
    fold_name, layer_color_ok, pattern_matches, pattern_problem,
};
use kentos_domain::{Document, NewLayer, Slot, default_style};

use crate::checks::{self, Stop, error};
use crate::{ExecutionContext, codes, set};

/// The step's name.
pub const TRANSLATE: &str = "Katmanları çevir";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn check_input(input: &LayersTranslate) -> Result<(), Stop> {
    if input.rules.is_empty() {
        return Err(fail(
            codes::NO_RULES,
            "En az bir çevirme kuralı verin (rules): hangi katmanların (from) hangi katmana (to) gideceği.".into(),
            "rules",
        ));
    }
    for (i, r) in input.rules.iter().enumerate() {
        if checks::is_blank(&r.to) {
            return Err(fail(
                codes::EMPTY_NAME,
                "Standart katmanın adı boş olamaz; bir ad yazın.".into(),
                &format!("rules[{i}].to"),
            ));
        }
        if let Some(problem) = pattern_problem(&r.from) {
            return Err(fail(
                codes::INVALID_PATTERN,
                format!(
                    "“{}” bir katman adı örüntüsü değil: {problem}. * herhangi bir dizi, ? tek karakter, [..] küme olur.",
                    r.from
                ),
                &format!("rules[{i}].from"),
            ));
        }
        if let Some(color) = r.color.as_deref()
            && !layer_color_ok(color)
        {
            return Err(fail(
                codes::INVALID_COLOR,
                format!(
                    "“{color}” bir katman rengi değil; #rrggbb ya da fg, fg-dim, ink, paper yazın."
                ),
                &format!("rules[{i}].color"),
            ));
        }
        checks::line_weight(r.line_weight, &format!("rules[{i}].lineWeight"))?;
    }
    Ok(())
}

/// What the rules do to this drawing: the sources by rule, the standard
/// layers found (none: to add).
struct Plan<'a> {
    /// Per rule: its standard layer, when the drawing has it.
    targets: Vec<Option<&'a LayerNode>>,
    /// The sources in the tree's order, with their rule.
    sources: Vec<(&'a LayerNode, usize)>,
}

fn holding<'a>(nodes: &'a [LayerNode], out: &mut Vec<&'a LayerNode>) {
    for n in nodes {
        if n.kind == LayerNodeType::Layer {
            out.push(n);
        }
        holding(&n.children, out);
    }
}

fn plan_of<'a>(doc: &'a Document, rules: &[TranslateRule]) -> Plan<'a> {
    let mut layers = Vec::new();
    holding(doc.layers().nodes(), &mut layers);
    let targets = rules
        .iter()
        .map(|r| {
            let to = fold_name(r.to.trim());
            layers.iter().copied().find(|n| fold_name(&n.name) == to)
        })
        .collect();
    let sources = layers
        .iter()
        .copied()
        .filter(|n| n.service.is_none())
        .filter_map(|n| {
            let rule = rules
                .iter()
                .position(|r| pattern_matches(&r.from, &n.name))?;
            (fold_name(&n.name) != fold_name(rules[rule].to.trim())).then_some((n, rule))
        })
        .collect();
    Plan { targets, sources }
}

fn check_doc(doc: &Document, input: &LayersTranslate) -> Result<(), Stop> {
    if let Some(parent) = input.parent.as_deref() {
        let Some(group) = doc.layers().get(parent) else {
            return Err(fail(
                codes::LAYER_NOT_FOUND,
                format!(
                    "“{parent}” kimlikli katman ya da grup çizimde yok. Var olan bir grubun kimliğini verin."
                ),
                "parent",
            ));
        };
        if group.kind != LayerNodeType::Group {
            return Err(fail(
                codes::NOT_A_GROUP,
                format!(
                    "“{}” bir katman; yeni katmanlar yalnız bir grubun içine eklenir. Bir grubun kimliğini verin ya da parent'ı vermeyin (en üst).",
                    group.name
                ),
                "parent",
            ));
        }
    }
    let plan = plan_of(doc, &input.rules);
    let used: Vec<usize> = {
        let mut u: Vec<usize> = plan.sources.iter().map(|(_, r)| *r).collect();
        u.sort_unstable();
        u.dedup();
        u
    };
    for &r in &used {
        if let Some(t) = plan.targets[r] {
            if t.service.is_some() {
                return Err(fail(
                    codes::SERVICE_LAYER,
                    format!(
                        "“{}” servisten çizilir ve nesne tutmaz; katmanlar ona çevrilemez. Kurala başka bir ad verin.",
                        t.name
                    ),
                    &format!("rules[{r}].to"),
                ));
            }
            if doc.layers().is_locked(&t.id) {
                return Err(fail(
                    codes::LAYER_LOCKED,
                    format!(
                        "Standart katman “{}” kilitli; katmanlar ona çevrilemez. Kilidini Katmanlar panelinden açın.",
                        t.name
                    ),
                    &format!("rules[{r}].to"),
                ));
            }
        }
    }
    if let Some((n, r)) = plan
        .sources
        .iter()
        .find(|(n, _)| doc.layers().is_locked(&n.id))
    {
        return Err(fail(
            codes::LAYER_LOCKED,
            format!(
                "“{}” katmanı kilitli; çevrilemez. Kilidini Katmanlar panelinden açın ya da kuralı (“{}”) daraltın.",
                n.name, input.rules[*r].from
            ),
            &format!("rules[{r}].from"),
        ));
    }
    Ok(())
}

pub(crate) fn check(doc: &Document, input: &LayersTranslate) -> Result<(), Stop> {
    check_input(input)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    check_doc(doc, input)
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &LayersTranslate) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(()) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The preview: the same write on a reading copy; the drawing stays.
pub fn plan(cx: &ExecutionContext<'_>, input: &LayersTranslate) -> CommandResult<LayersTranslated> {
    if let Err(stop) = check(cx.doc, input) {
        return stop.into();
    }
    let mut copy = cx.doc.reading_copy();
    match write(&mut copy, input) {
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

/// Translates the layers as one undo step.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: LayersTranslate,
) -> CommandResult<LayersTranslated> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_LAYERS_TRANSLATE,
        kentos_contracts::CAD_LAYERS_TRANSLATE_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: LayersTranslate,
) -> CommandResult<LayersTranslated> {
    if let Err(stop) = check(cx.doc, &input) {
        return stop.into();
    }
    write(cx.doc, &input)
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// A source layer's id, name and rule.
type Source = (String, String, usize);
/// A rule's standard layer, when the drawing has it: its id and look.
type Target = Option<(String, kentos_contracts::LayerStyle)>;

/// Translates the layers (its checks passed); `cad.standards.apply` writes its merging so.
pub(crate) fn write(doc: &mut Document, input: &LayersTranslate) -> CommandResult<LayersTranslated> {
    // What goes where, read before anything changes.
    let (sources, targets): (Vec<Source>, Vec<Target>) = {
        let plan = plan_of(doc, &input.rules);
        (
            plan.sources
                .iter()
                .map(|(n, r)| (n.id.clone(), n.name.clone(), *r))
                .collect(),
            plan.targets
                .iter()
                .map(|t| t.map(|t| (t.id.clone(), t.style.clone())))
                .collect(),
        )
    };
    let mut out = LayersTranslated {
        changed: false,
        translations: Vec::new(),
        created: Vec::new(),
        restyled: Vec::new(),
        objects: 0,
        revision: doc.revision().to_string(),
    };
    if sources.is_empty() {
        return CommandResult::Completed {
            output: out,
            warnings: Vec::new(),
        };
    }
    let mut per_layer: HashMap<String, Vec<Slot>> = HashMap::new();
    for e in doc.entities() {
        per_layer
            .entry(e.base().layer_id.clone())
            .or_default()
            .push(Slot(e.base().id));
    }
    let group = doc.begin_group(TRANSLATE);
    // Each used rule's standard layer: found and restyled, or added.
    let mut standard: HashMap<usize, (String, String)> = HashMap::new();
    for (source, _, r) in &sources {
        if standard.contains_key(r) {
            continue;
        }
        let rule = &input.rules[*r];
        let given = |mut style: kentos_contracts::LayerStyle| {
            if let Some(c) = &rule.color {
                style.color.clone_from(c);
            }
            if let Some(t) = rule.line_type {
                style.line_type = t;
            }
            if let Some(w) = rule.line_weight {
                style.line_weight = w;
            }
            style
        };
        match &targets[*r] {
            Some((id, style)) => {
                let next = given(style.clone());
                if &next != style && doc.set_layer_style(id, next, TRANSLATE) {
                    out.restyled.push(id.clone());
                }
                let name = doc
                    .layers()
                    .get(id)
                    .map(|n| n.name.clone())
                    .unwrap_or_default();
                standard.insert(*r, (id.clone(), name));
            }
            None => {
                let base = doc
                    .layers()
                    .get(source)
                    .map_or_else(default_style, |n| n.style.clone());
                let mut new = NewLayer::layer(rule.to.trim());
                new.style = given(kentos_contracts::LayerStyle {
                    fill: None,
                    point: None,
                    label: None,
                    labels: None,
                    pick_interior: None,
                    renderer: None,
                    ..base
                });
                match doc.add_layer_at(new, input.parent.as_deref(), None, None, false) {
                    Ok(id) => {
                        out.created.push(id.clone());
                        standard.insert(*r, (id, rule.to.trim().to_owned()));
                    }
                    Err(r) => {
                        doc.cancel_group(group);
                        return Stop::failed(error(
                            codes::LAYER_REFUSED,
                            r.0,
                            Some("rules".into()),
                        ))
                        .into();
                    }
                }
            }
        }
    }
    let mut warnings: Vec<CommandWarning> = Vec::new();
    let active = doc.layers().active().to_owned();
    // The objects go to each standard layer in one `cad.entities.set`: a call per source would read the drawing again for each.
    let mut by_target: Vec<(String, Vec<String>)> = Vec::new();
    for (source, name, r) in &sources {
        let (to, to_name) = standard[r].clone();
        let slots = per_layer.remove(source).unwrap_or_default();
        let moved = slots.len();
        let uids = slots
            .iter()
            .filter_map(|s| doc.uid(*s))
            .map(|u| u.to_string());
        match by_target.iter_mut().find(|(t, _)| *t == to) {
            Some((_, list)) => list.extend(uids),
            None => by_target.push((to.clone(), uids.collect())),
        }
        out.objects += count(moved);
        out.translations.push(LayerTranslation {
            layer: source.clone(),
            name: name.clone(),
            rule: count(*r),
            to,
            to_name,
            objects: count(moved),
        });
    }
    for (to, uids) in by_target {
        if uids.is_empty() {
            continue;
        }
        let set_input = EntitiesSetProperties {
            line_type: None,
            line_type_scale: None,
            transparency: None,
            wipeout: None,
            uids,
            layer_id: Some(to),
            color: None,
            line_weight: None,
            symbol: None,
            attrs: None,
            label: None,
            operation: PropertiesOperation::Layer,
            expected_revision: None,
            unlink: false,
            attachments: None,
        };
        match set::execute(&mut ExecutionContext::new(doc), set_input) {
            CommandResult::Completed { warnings: w, .. } => {
                for warning in w {
                    if !warnings.iter().any(|x| x.code == warning.code) {
                        warnings.push(warning);
                    }
                }
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
    // The sources removed in one transaction: each removal its own step would end with the drawing read again.
    let removed = doc.transact(TRANSLATE, |doc| {
        for (source, _, r) in &sources {
            if *source == active {
                doc.set_active_layer(&standard[r].0);
            }
            doc.remove_layer(source)?;
        }
        Ok::<(), kentos_domain::Refusal>(())
    });
    if let Err(refusal) = removed {
        doc.cancel_group(group);
        return Stop::failed(error(codes::LAYER_REFUSED, refusal.0, Some("rules".into()))).into();
    }
    doc.end_group(group);
    out.changed = true;
    out.revision = doc.revision().to_string();
    CommandResult::Completed {
        output: out,
        warnings,
    }
}
