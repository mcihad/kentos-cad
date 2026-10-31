//! A form's preflight (docs/adr/0401 §4): what keeps a form from being used, said before a record
//! is touched. The designer shows it as the form is drawn; publishing and data entry refuse a form
//! with an error.

use kentos_contracts::{
    FormBody, FormField, FormIssue, FormItem, FormSeverity, FormWidget, LayerField, LayerFieldKind,
    body_problem, check_value,
};
use kentos_expression::js::text::fold_turkish;
use kentos_expression::{Expr, Variable, compile_with, template};

use crate::lookup::World;
use crate::session::{CHOICE_CODE, CHOICE_LABEL, MOMENT, reads_moment, still_schema};
use crate::{codes, issue, record::schema_of, widget_of};

/// What is wrong with `body` over the layer's `fields`, errors and warnings, in the form's order. A
/// list from another layer or a related item is said missing: [`preflight_in`] knows the others.
pub fn preflight(body: &FormBody, fields: &[LayerField], variables: &[Variable]) -> Vec<FormIssue> {
    preflight_in(body, fields, variables, &World::new())
}

/// [`preflight`] knowing the project's other layers (their fields): the lists from them and the
/// related items are checked against them (docs/adr/0404 §2).
pub fn preflight_in(
    body: &FormBody,
    fields: &[LayerField],
    variables: &[Variable],
    world: &World,
) -> Vec<FormIssue> {
    if let Some(problem) = body_problem(body) {
        return vec![issue(
            codes::BODY_INVALID,
            FormSeverity::Error,
            None,
            None,
            None,
            problem,
        )];
    }
    let schema = crate::record::form_schema_in(fields, variables, kentos_expression::Unit::METRE);
    let mut p = Preflight {
        fields,
        world,
        variables,
        still: still_schema(&schema),
        schema,
        issues: Vec::new(),
        bound: Vec::new(),
        mirrors: Vec::new(),
        calcs: Vec::new(),
    };
    p.items(&body.items, false);
    // A mirror shows a field the form writes elsewhere (docs/adr/0437).
    for (item, key) in std::mem::take(&mut p.mirrors) {
        if !p.bound.iter().any(|b| b.key == key) {
            p.issues.push(issue(
                codes::MIRROR_ALONE,
                FormSeverity::Error,
                Some(&item),
                Some(&key),
                None,
                format!("“{key}” alanının yansıması var ama alanın kendi öğesi formda yok; alanı ekleyin."),
            ));
        }
    }
    p.calc_cycle();
    // A relation `ilişkili_toplam` names that the project has not (docs/adr/0761): its value is empty.
    for name in crate::named_relations(body) {
        if !crate::relatives::in_world(&name, world) {
            p.issues.push(issue(
                codes::RELATION_UNKNOWN,
                FormSeverity::Warning,
                None,
                None,
                None,
                format!(
                    "ilişkili_toplam “{name}” ilişkisini anıyor ama projede bu adla ya da kimlikle bir ilişki yok; değeri boş kalır."
                ),
            ));
        }
    }
    // The scope over the layer's fields (docs/adr/0539).
    if let Some(problem) =
        crate::scope::scope_problem(body, fields, variables, kentos_expression::Unit::METRE)
    {
        p.issues.push(issue(
            codes::SCOPE_INVALID,
            FormSeverity::Error,
            None,
            None,
            Some("scope"),
            problem,
        ));
    }
    // The workflow over the layer's fields (docs/adr/0428 §1).
    if let Some(problem) = body
        .workflow
        .as_ref()
        .and_then(|w| kentos_contracts::workflow_problem(w, fields))
    {
        p.issues.push(issue(
            codes::WORKFLOW_INVALID,
            FormSeverity::Error,
            None,
            None,
            Some("workflow"),
            problem,
        ));
    }
    // An action's step, the workflow's (docs/adr/0442 §5).
    for (_, item) in body.walk() {
        if let FormItem::Action {
            id,
            kind: kentos_contracts::ActionKind::Step,
            step: Some(step),
            ..
        } = item
            && !body
                .workflow
                .as_ref()
                .is_some_and(|w| w.steps.iter().any(|s| s.label == *step))
        {
            p.issues.push(issue(
                codes::ACTION_STEP_UNKNOWN,
                FormSeverity::Error,
                Some(id),
                None,
                Some("step"),
                format!(
                    "“{step}” adımı formun iş akışında yok; eylemi var olan bir adıma bağlayın."
                ),
            ));
        }
    }
    // A translation's words for an item the form has not: kept, said (docs/adr/0443 §5).
    let ids: std::collections::HashSet<&str> = body.walk().iter().map(|(_, i)| i.id()).collect();
    for (tag, words) in &body.translations {
        for id in words.items.keys().filter(|id| !ids.contains(id.as_str())) {
            p.issues.push(issue(
                codes::TRANSLATION_ITEM,
                FormSeverity::Warning,
                None,
                None,
                None,
                format!("“{tag}” çevirisinde “{id}” öğesinin sözleri var ama formda bu öğe yok."),
            ));
        }
    }
    if p.bound.is_empty() {
        p.issues.push(issue(
            codes::EMPTY,
            FormSeverity::Warning,
            None,
            None,
            None,
            "Formda alan yok; kayıt bu formla düzenlenemez.".into(),
        ));
    }
    // A field the layer requires: the form shows it always and lets it be written.
    for f in fields.iter().filter(|f| f.required) {
        let reach = p.bound.iter().find(|b| b.key == f.key());
        let message = match reach {
            None => Some(format!(
                "“{}” alanı katmanda zorunlu ama formda yok; ekleyin.",
                f.label()
            )),
            Some(b) if !b.always => Some(format!(
                "“{}” alanı katmanda zorunlu; form onu bir kuralla gizleyebilir ya da kilitleyebilir.",
                f.label()
            )),
            _ => None,
        };
        if let Some(message) = message {
            p.issues.push(issue(
                codes::REQUIRED_HIDDEN,
                FormSeverity::Error,
                reach.map(|b| b.item.as_str()),
                Some(f.key()),
                None,
                message,
            ));
        }
    }
    p.issues
}

/// Whether `widget` edits a field of `field`'s kind; the list widgets also need a value list (a
/// yes/no field's two values count as one).
pub fn widget_fits(widget: FormWidget, field: &LayerField) -> Result<(), &'static str> {
    use FormWidget as W;
    let listed = field.values.as_ref().is_some_and(|v| !v.is_empty());
    let kind_fits = match field.kind {
        LayerFieldKind::Text => matches!(
            widget,
            W::Text
                | W::TextArea
                | W::List
                | W::Radio
                | W::Segmented
                | W::Color
                | W::CheckList
                | W::Search
                | W::Chips
                | W::Tree
        ),
        LayerFieldKind::Integer | LayerFieldKind::Decimal => {
            matches!(
                widget,
                W::Number | W::List | W::Radio | W::Segmented | W::Slider | W::Search | W::Tree
            )
        }
        LayerFieldKind::Date | LayerFieldKind::DateTime | LayerFieldKind::Time => {
            matches!(widget, W::Date | W::Text)
        }
        LayerFieldKind::Boolean => matches!(widget, W::Check | W::Switch | W::Radio | W::Segmented),
        // docs/adr/0629 §3: written as text; a big integer's list as a number's, without a slider (no float).
        LayerFieldKind::BigInt => {
            matches!(
                widget,
                W::Text | W::List | W::Radio | W::Segmented | W::Search | W::Tree
            )
        }
        LayerFieldKind::Instant | LayerFieldKind::Uuid | LayerFieldKind::Binary => {
            matches!(widget, W::Text)
        }
        LayerFieldKind::Json => matches!(widget, W::Text | W::TextArea),
    };
    // A listed multiple field is edited by its check list, a free one by text; the check list edits
    // only a multiple field (docs/adr/0408 §2, 0409 §1).
    // Chips edit a listed multiple field too (docs/adr/0431).
    let many_fits = if field.multiple && listed {
        matches!(widget, W::CheckList | W::Chips)
    } else if field.multiple {
        matches!(widget, W::Text | W::TextArea | W::CheckList | W::Chips)
    } else {
        !matches!(widget, W::CheckList | W::Chips)
    };
    if !kind_fits || !many_fits {
        return Err(codes::WIDGET_UNFIT);
    }
    // A slider runs between the field's least and most (docs/adr/0406 §2).
    if widget == W::Slider && (field.min.is_none() || field.max.is_none()) {
        return Err(codes::WIDGET_NEEDS_RANGE);
    }
    let needs_list = matches!(
        widget,
        W::List | W::Radio | W::Segmented | W::CheckList | W::Search | W::Chips | W::Tree
    );
    if needs_list && !listed && field.kind != LayerFieldKind::Boolean {
        return Err(codes::WIDGET_NEEDS_VALUES);
    }
    Ok(())
}

/// A field the form shows, and whether it is always shown and writable.
struct Bound {
    key: String,
    item: String,
    always: bool,
}

struct Preflight<'a> {
    fields: &'a [LayerField],
    world: &'a World,
    variables: &'a [Variable],
    schema: kentos_expression::Schema,
    /// The schema without the moment's and the user's values.
    still: kentos_expression::Schema,
    issues: Vec<FormIssue>,
    bound: Vec<Bound>,
    /// The mirrors (docs/adr/0437): their item and their field's key.
    mirrors: Vec<(String, String)>,
    /// Each calculated field: its item, its key, its layer field's name and the names it reads.
    calcs: Vec<(String, String, String, Vec<String>)>,
}

impl Preflight<'_> {
    /// `hideable`: an enclosing group has a rule that may hide it.
    fn items(&mut self, items: &[FormItem], hideable: bool) {
        for item in items {
            match item {
                FormItem::Tab {
                    id,
                    items,
                    visible_when,
                    ..
                } => {
                    if let Some(rule) = visible_when {
                        self.rule(id, None, None, "visibleWhen", rule);
                    }
                    self.items(items, hideable || visible_when.is_some());
                }
                FormItem::Group {
                    id,
                    visible_when,
                    items,
                    ..
                } => {
                    if let Some(rule) = visible_when {
                        self.rule(id, None, None, "visibleWhen", rule);
                    }
                    self.items(items, hideable || visible_when.is_some());
                }
                FormItem::Field(f) => self.field(f, hideable),
                FormItem::Note { id, text } => self.note(id, text),
                FormItem::Related {
                    id,
                    layer,
                    field,
                    visible_when,
                    columns,
                    ..
                } => {
                    if let Some(rule) = visible_when {
                        self.rule(id, None, None, "visibleWhen", rule);
                    }
                    self.related(id, layer, field, columns);
                }
                FormItem::Attachments {
                    id, visible_when, ..
                }
                | FormItem::Location {
                    id, visible_when, ..
                }
                | FormItem::Divider {
                    id, visible_when, ..
                }
                | FormItem::Action {
                    id, visible_when, ..
                } => {
                    if let Some(rule) = visible_when {
                        self.rule(id, None, None, "visibleWhen", rule);
                    }
                }
            }
        }
    }

    fn field(&mut self, f: &FormField, hideable: bool) {
        let at = |code: &str, rule: Option<&str>, message: String| {
            issue(
                code,
                FormSeverity::Error,
                Some(&f.id),
                Some(&f.field),
                rule,
                message,
            )
        };
        if f.mirror {
            self.mirrors.push((f.id.clone(), f.field.clone()));
            if !self.fields.iter().any(|l| l.key() == f.field) {
                self.issues.push(at(
                    codes::FIELD_UNKNOWN,
                    None,
                    format!("Katmanda “{}” alanı yok.", f.field),
                ));
            }
            if f.writes_rules() {
                self.issues.push(at(
                    codes::MIRROR_RULES,
                    None,
                    format!(
                        "“{}” bir yansıma: yalnız başlığı, yardımı, görünüşü ve görünme koşulu olur; kuralları alanın kendi öğesine yazın.",
                        f.field
                    ),
                ));
            }
            if let Some(rule) = &f.visible_when {
                self.rule(&f.id, Some(&f.field), None, "visibleWhen", rule);
            }
            return;
        }
        let layer = self.fields.iter().find(|l| l.key() == f.field);
        if self.bound.iter().any(|b| b.key == f.field) {
            self.issues.push(at(
                codes::FIELD_TWICE,
                None,
                format!("“{}” alanı formda iki kez var.", f.field),
            ));
        }
        // A calculated field is written by its calculation: it is filled though read-only.
        let always = !hideable
            && f.visible_when.is_none()
            && (f.calculate.is_some() || (!f.read_only && f.read_only_when.is_none()));
        self.bound.push(Bound {
            key: f.field.clone(),
            item: f.id.clone(),
            always,
        });
        let Some(layer) = layer else {
            self.issues.push(at(
                codes::FIELD_UNKNOWN,
                None,
                format!("Katmanda “{}” alanı yok.", f.field),
            ));
            return;
        };
        let widget = widget_of(f, layer);
        // A list from another layer is a value list for the widgets.
        let mut probe = layer.clone();
        if f.lookup.is_some() {
            probe.values = Some(vec![kentos_contracts::FieldChoice {
                code: "-".into(),
                label: "-".into(),
                retired: false,
                description: None,
                from: None,
                until: None,
            }]);
        }
        if let Err(code) = widget_fits(widget, &probe) {
            let message = if code == codes::WIDGET_NEEDS_RANGE {
                format!(
                    "Kaydırıcı en az ve en çok değer ister; “{}” alanının aralığı yok.",
                    layer.label()
                )
            } else if code == codes::WIDGET_UNFIT {
                format!(
                    "“{}” bir {} alanı; {} onu düzenleyemez.",
                    layer.label(),
                    layer.kind.label().to_lowercase(),
                    widget.label().to_lowercase()
                )
            } else {
                format!(
                    "{} değer listesi ister; “{}” alanının listesi yok.",
                    widget.label(),
                    layer.label()
                )
            };
            self.issues.push(at(code, None, message));
        }
        if let Some(default) = &f.default {
            let mut loose = layer.clone();
            loose.required = false;
            if let Err(r) = check_value(&loose, default) {
                self.issues
                    .push(at(codes::DEFAULT_INVALID, Some("default"), r.message));
            } else if widget == FormWidget::Color && !crate::is_colour(default) {
                self.issues.push(at(
                    codes::DEFAULT_INVALID,
                    Some("default"),
                    crate::colour_message(layer.label(), default),
                ));
            }
        }
        // Suggestions are a text field's own values; a list offers its values instead.
        if f.suggest
            && (layer.kind != LayerFieldKind::Text
                || layer.values.as_ref().is_some_and(|v| !v.is_empty())
                || f.lookup.is_some())
        {
            self.issues.push(at(
                codes::SUGGEST_UNFIT,
                Some("suggest"),
                format!(
                    "“{}” için öneri yalnız değer listesi olmayan metin alanında olur.",
                    layer.label()
                ),
            ));
        }
        if f.default.is_some() && f.default_expr.is_some() {
            self.issues.push(at(
                codes::RULE_CONFLICT,
                Some("defaultExpr"),
                format!(
                    "“{}” alanının hem değer hem ifade varsayılanı var; birini bırakın.",
                    layer.label()
                ),
            ));
        }
        if f.calculate.is_some()
            && (f.default.is_some() || f.default_expr.is_some() || f.read_only_when.is_some())
        {
            self.issues.push(at(
                codes::RULE_CONFLICT,
                Some("calculate"),
                format!(
                    "“{}” hesaplanan bir alan; varsayılanı ve kilit kuralı olmaz.",
                    layer.label()
                ),
            ));
        }
        if f.overridable && f.calculate.is_none() {
            self.issues.push(at(
                codes::RULE_CONFLICT,
                Some("overridable"),
                format!(
                    "“{}” alanının hesabı yok; değiştirilebilir hesap ancak hesapla olur.",
                    layer.label()
                ),
            ));
        }
        if f.choice_filter.is_some() && layer.values.as_ref().is_none_or(|v| v.is_empty()) {
            self.issues.push(at(
                codes::CHOICE_UNLISTED,
                Some("choiceFilter"),
                format!(
                    "“{}” alanının değer listesi yok; seçenek süzgeci onu süzemez.",
                    layer.label()
                ),
            ));
        }
        if let Some(lookup) = &f.lookup {
            self.lookup(f, layer, lookup);
        }
        if f.read_only
            && f.required
            && f.default.is_none()
            && f.default_expr.is_none()
            && f.calculate.is_none()
        {
            self.issues.push(at(
                codes::REQUIRED_READ_ONLY,
                None,
                format!(
                    "“{}” hem zorunlu hem salt okunur; varsayılanı olmadan doldurulamaz.",
                    layer.label()
                ),
            ));
        }
        let own = Some(layer.name.as_str());
        for (rule, text) in [
            ("visibleWhen", &f.visible_when),
            ("requiredWhen", &f.required_when),
            ("readOnlyWhen", &f.read_only_when),
        ] {
            if let Some(text) = text {
                let itself = if rule == "requiredWhen" { None } else { own };
                self.rule(&f.id, Some(&f.field), itself, rule, text);
            }
        }
        for (i, c) in f.checks.iter().enumerate() {
            self.rule(&f.id, Some(&f.field), None, &format!("checks/{i}"), &c.rule);
        }
        for (rule, text) in [
            ("defaultExpr", &f.default_expr),
            ("calculate", &f.calculate),
        ] {
            let Some(text) = text else { continue };
            let expr = self.rule(&f.id, Some(&f.field), None, rule, text);
            if expr.is_some() && reads_moment(text, &self.still) {
                self.issues.push(issue(
                    codes::MOMENT_ONLY,
                    FormSeverity::Warning,
                    Some(&f.id),
                    Some(&f.field),
                    Some(rule),
                    format!(
                        "“{}” alanının {} anı ya da kullanıcıyı okuyor; yalnız formla yazmada hesaplanır, çizimde ve içe aktarmada boş kalır.",
                        layer.label(),
                        if rule == "calculate" { "hesabı" } else { "varsayılanı" }
                    ),
                ));
            }
            if rule == "calculate"
                && let Some(e) = expr
            {
                self.calcs
                    .push((f.id.clone(), f.field.clone(), layer.name.clone(), e.fields));
            }
        }
        if let Some(text) = &f.choice_filter {
            let mut schema = self.schema.clone();
            for name in [CHOICE_CODE, CHOICE_LABEL] {
                schema.variables.push(Variable {
                    name: name.into(),
                    value: kentos_expression::Value::text(String::new()),
                    description: String::new(),
                });
            }
            let kept = std::mem::replace(&mut self.schema, schema);
            self.rule(&f.id, Some(&f.field), None, "choiceFilter", text);
            self.schema = kept;
        }
    }

    /// A field's list from another layer (docs/adr/0404 §2): its layer and fields exist, it is not
    /// with the field's own list or a choice filter, its filter compiles over its layer.
    fn lookup(&mut self, f: &FormField, layer: &LayerField, lookup: &kentos_contracts::FormLookup) {
        let at = |code: &str, rule: &str, message: String| {
            issue(
                code,
                FormSeverity::Error,
                Some(&f.id),
                Some(&f.field),
                Some(rule),
                message,
            )
        };
        if layer.values.as_ref().is_some_and(|v| !v.is_empty()) || f.choice_filter.is_some() {
            self.issues.push(at(
                codes::RULE_CONFLICT,
                "lookup",
                format!(
                    "“{}” alanının hem kendi değer listesi ya da seçenek süzgeci hem başka katmandan listesi var; birini bırakın.",
                    layer.label()
                ),
            ));
        }
        let Some(other) = self.world.get(&lookup.layer) else {
            self.issues.push(at(
                codes::LOOKUP_LAYER,
                "lookup",
                format!(
                    "“{}” alanının listesinin katmanı (“{}”) projede yok; başka bir katman seçin.",
                    layer.label(),
                    lookup.layer
                ),
            ));
            return;
        };
        let has = |key: &str| other.fields.iter().any(|x| x.key() == key);
        let mut missing: Vec<&str> = Vec::new();
        for key in [Some(lookup.key.as_str()), lookup.label.as_deref()]
            .into_iter()
            .flatten()
        {
            if key != kentos_contracts::LOOKUP_UID && !has(key) {
                missing.push(key);
            }
        }
        for m in &lookup.matches {
            if !has(&m.field) {
                missing.push(&m.field);
            }
        }
        for key in missing {
            self.issues.push(at(
                codes::LOOKUP_FIELD,
                "lookup",
                format!("“{}” katmanında “{key}” alanı yok.", other.name),
            ));
        }
        for m in &lookup.matches {
            if !self.fields.iter().any(|x| x.key() == m.record) {
                self.issues.push(at(
                    codes::LOOKUP_FIELD,
                    "lookup",
                    format!("Formun katmanında “{}” alanı yok.", m.record),
                ));
            }
        }
        if let Some(src) = &lookup.filter {
            let schema = schema_of(&other.fields, self.variables);
            if let Err(e) = compile_with(src, &schema) {
                self.issues.push(at(
                    codes::RULE_INVALID,
                    "lookup.filter",
                    format!("Listenin süzgeci okunamadı: {}", e.text()),
                ));
            }
        }
        self.lookup_extras(f, layer, lookup, other);
    }

    /// A list's order, columns, page, copies, stamp and geometry (docs/adr/0439): their fields exist,
    /// the kinds it compares, writes and copies go together, its numbers are in bounds.
    fn lookup_extras(
        &mut self,
        f: &FormField,
        layer: &LayerField,
        lookup: &kentos_contracts::FormLookup,
        other: &crate::OtherLayer,
    ) {
        let at = |code: &str, message: String| {
            issue(
                code,
                FormSeverity::Error,
                Some(&f.id),
                Some(&f.field),
                Some("lookup"),
                message,
            )
        };
        let theirs = |key: &str| other.fields.iter().find(|x| x.key() == key);
        let ours = |key: &str| self.fields.iter().find(|x| x.key() == key);
        let mut issues = Vec::new();
        // Their fields the list reads beyond its key, label and chain.
        let mut read: Vec<&str> = lookup.columns.iter().map(String::as_str).collect();
        read.extend(lookup.sort.as_ref().map(|s| s.field.as_str()));
        read.extend(lookup.copy.iter().map(|c| c.from.as_str()));
        for key in read {
            if key != kentos_contracts::LOOKUP_UID && theirs(key).is_none() {
                issues.push(at(
                    codes::LOOKUP_FIELD,
                    format!("“{}” katmanında “{key}” alanı yok.", other.name),
                ));
            }
        }
        // Our fields the list writes.
        let mut written: Vec<&str> = lookup.copy.iter().map(|c| c.to.as_str()).collect();
        written.extend(lookup.stamp.as_deref());
        for key in written {
            if ours(key).is_none() {
                issues.push(at(
                    codes::LOOKUP_FIELD,
                    format!("Formun katmanında “{key}” alanı yok."),
                ));
            }
        }
        // The kinds: the key into the field, each match, each copy.
        let kind_of = |x: Option<&LayerField>, key: &str| {
            if key == kentos_contracts::LOOKUP_UID {
                Some(LayerFieldKind::Text)
            } else {
                x.map(|x| x.kind)
            }
        };
        if let Some(k) = kind_of(theirs(&lookup.key), &lookup.key)
            && !writes_into(k, layer.kind)
        {
            issues.push(at(
                codes::LOOKUP_TYPE,
                format!(
                    "“{}” alanı {} ister; listenin saklanan alanı {}. Aynı türden bir alan seçin.",
                    layer.label(),
                    kind_words(layer.kind),
                    kind_words(k)
                ),
            ));
        }
        for m in &lookup.matches {
            if let (Some(a), Some(b)) = (theirs(&m.field), ours(&m.record))
                && family(a.kind) != family(b.kind)
            {
                issues.push(at(
                    codes::LOOKUP_TYPE,
                    format!(
                        "Zincirde “{}” ({}) ile “{}” ({}) karşılaştırılamaz; aynı türden alanları eşleyin.",
                        a.label(),
                        kind_words(a.kind),
                        b.label(),
                        kind_words(b.kind)
                    ),
                ));
            }
        }
        for c in &lookup.copy {
            if let (Some(k), Some(b)) = (kind_of(theirs(&c.from), &c.from), ours(&c.to))
                && !writes_into(k, b.kind)
            {
                issues.push(at(
                    codes::LOOKUP_TYPE,
                    format!(
                        "“{}” alanına {} kopyalanamaz; “{}” {} ister.",
                        b.label(),
                        kind_words(k),
                        b.label(),
                        kind_words(b.kind)
                    ),
                ));
            }
            if c.to == f.field || Some(c.to.as_str()) == lookup.stamp.as_deref() {
                issues.push(at(
                    codes::LOOKUP_TYPE,
                    format!(
                        "“{}” alanı hem listenin değeri ya da kaynak damgası hem kopyanın hedefi olamaz.",
                        c.to
                    ),
                ));
            }
        }
        if let Some(b) = lookup.stamp.as_deref().and_then(ours)
            && b.kind != LayerFieldKind::Text
        {
            issues.push(at(
                codes::LOOKUP_TYPE,
                format!(
                    "Kaynak damgası metin alanına yazılır; “{}” metin değil.",
                    b.label()
                ),
            ));
        }
        if lookup.columns.len() > kentos_contracts::MOST_LOOKUP_COLUMNS {
            issues.push(at(
                codes::LOOKUP_TYPE,
                format!(
                    "Liste en çok {} sütun gösterir; {} verildi.",
                    kentos_contracts::MOST_LOOKUP_COLUMNS,
                    lookup.columns.len()
                ),
            ));
        }
        if lookup
            .limit
            .is_some_and(|n| n == 0 || n > kentos_contracts::MOST_LOOKUP_LIMIT)
        {
            issues.push(at(
                codes::LOOKUP_TYPE,
                format!(
                    "Listenin sayfası 1 ile {} arasında olmalı.",
                    kentos_contracts::MOST_LOOKUP_LIMIT
                ),
            ));
        }
        if lookup
            .spatial
            .as_ref()
            .and_then(|s| s.tolerance)
            .is_some_and(|t| !t.is_finite() || t < 0.0)
        {
            issues.push(at(
                codes::LOOKUP_TYPE,
                "Mekânsal listenin toleransı sıfır ya da artı bir sayı olmalı.".to_owned(),
            ));
        }
        self.issues.extend(issues);
    }

    /// A related item (docs/adr/0404 §4): its layer and its field exist.
    fn related(&mut self, id: &str, layer: &str, field: &str, columns: &[String]) {
        let at = |code: &str, message: String| {
            issue(
                code,
                FormSeverity::Error,
                Some(id),
                None,
                Some("related"),
                message,
            )
        };
        let Some(other) = self.world.get(layer) else {
            self.issues.push(at(
                codes::LOOKUP_LAYER,
                format!(
                    "İlişkili kayıtların katmanı (“{layer}”) projede yok; başka bir katman seçin."
                ),
            ));
            return;
        };
        if !other.fields.iter().any(|x| x.key() == field) {
            self.issues.push(at(
                codes::LOOKUP_FIELD,
                format!("“{}” katmanında “{field}” alanı yok.", other.name),
            ));
        }
        // The table's columns (docs/adr/0425).
        for c in columns {
            if !other.fields.iter().any(|x| x.key() == c) {
                self.issues.push(at(
                    codes::LOOKUP_FIELD,
                    format!(
                        "Tablonun sütunu: “{}” katmanında “{c}” alanı yok.",
                        other.name
                    ),
                ));
            }
        }
    }

    /// A note whose `{…}` put values in: its template splits and its expressions compile.
    fn note(&mut self, id: &str, text: &str) {
        match template::parse(text) {
            Err(e) => self.issues.push(issue(
                codes::RULE_INVALID,
                FormSeverity::Error,
                Some(id),
                None,
                Some("text"),
                format!("Notun yazısı okunamadı: {}", e.text()),
            )),
            Ok(t) => {
                let sources: Vec<String> = t.expressions().map(|(s, _)| s.to_owned()).collect();
                for src in sources {
                    self.rule(id, None, None, "text", &src);
                }
            }
        }
    }

    /// Calculated fields that read each other in a circle: the first of the circle in the form's
    /// order is said.
    fn calc_cycle(&mut self) {
        let n = self.calcs.len();
        let deps: Vec<Vec<usize>> = self
            .calcs
            .iter()
            .map(|(_, _, _, reads)| {
                (0..n)
                    .filter(|&k| reads.contains(&self.calcs[k].2))
                    .collect()
            })
            .collect();
        let mut done = vec![false; n];
        while let Some(next) = (0..n).find(|&k| !done[k] && deps[k].iter().all(|&d| done[d])) {
            done[next] = true;
        }
        if let Some(k) = (0..n).find(|&k| !done[k]) {
            let (item, key, name, _) = &self.calcs[k];
            self.issues.push(issue(
                codes::CALC_CYCLE,
                FormSeverity::Error,
                Some(item),
                Some(key),
                Some("calculate"),
                format!(
                    "“{name}” alanının hesabı kendisini ya da onu okuyan bir hesabı okuyor; döngüyü kırın."
                ),
            ));
        }
    }

    /// A rule compiles over the layer's fields; `itself`: the field it must not read.
    fn rule(
        &mut self,
        item: &str,
        key: Option<&str>,
        itself: Option<&str>,
        at: &str,
        text: &str,
    ) -> Option<Expr> {
        let expr = match compile_with(text, &self.schema) {
            Ok(e) => e,
            Err(e) => {
                self.issues.push(issue(
                    codes::RULE_INVALID,
                    FormSeverity::Error,
                    Some(item),
                    key,
                    Some(at),
                    format!("Kural okunamadı: {}", e.text()),
                ));
                return None;
            }
        };
        let warn = |code: &str, message: String| {
            issue(
                code,
                FormSeverity::Warning,
                Some(item),
                key,
                Some(at),
                message,
            )
        };
        for name in &expr.fields {
            if !self.fields.iter().any(|f| &f.name == name) {
                self.issues.push(warn(
                    codes::RULE_UNKNOWN_NAME,
                    format!("Kural katmanda olmayan “{name}” adını okuyor; değeri boş okunur."),
                ));
            } else if itself == Some(name.as_str()) {
                self.issues.push(warn(
                    codes::RULE_READS_ITSELF,
                    "Kural alanın kendi değerini okuyor; yazarken alan gizlenebilir ya da kilitlenebilir."
                        .to_owned(),
                ));
            }
        }
        // The moment's and the user's values are a command's to give.
        for name in expr
            .unknown_variables
            .iter()
            .filter(|n| !MOMENT.contains(&fold_turkish(n).to_lowercase().as_str()))
        {
            self.issues.push(warn(
                codes::RULE_UNKNOWN_VARIABLE,
                format!("Projede “@{name}” değeri yok; boş okunur."),
            ));
        }
        Some(expr)
    }
}

/// A field kind's family: what compares with what (docs/adr/0439 §4).
fn family(kind: LayerFieldKind) -> u8 {
    match kind {
        LayerFieldKind::Text => 0,
        LayerFieldKind::Integer | LayerFieldKind::Decimal | LayerFieldKind::BigInt => 1,
        LayerFieldKind::Date => 2,
        LayerFieldKind::DateTime => 3,
        LayerFieldKind::Time => 4,
        LayerFieldKind::Boolean => 5,
        LayerFieldKind::Instant => 6,
        LayerFieldKind::Uuid => 7,
        LayerFieldKind::Json => 8,
        LayerFieldKind::Binary => 9,
    }
}

/// Whether a value of kind `from` may be written into a field of kind `to`: the same family (an
/// integer into a decimal, not a decimal into an integer), or any into text.
fn writes_into(from: LayerFieldKind, to: LayerFieldKind) -> bool {
    to == LayerFieldKind::Text
        || (family(from) == family(to)
            && !(from == LayerFieldKind::Decimal && to == LayerFieldKind::Integer)
            // A 64-bit whole number fits neither a 2⁵³ one nor takes a decimal (docs/adr/0629 §2).
            && !(from == LayerFieldKind::BigInt && to == LayerFieldKind::Integer)
            && !(from == LayerFieldKind::Decimal && to == LayerFieldKind::BigInt))
}

/// A field kind in words.
fn kind_words(kind: LayerFieldKind) -> &'static str {
    match kind {
        LayerFieldKind::Text => "metin",
        LayerFieldKind::Integer => "tam sayı",
        LayerFieldKind::Decimal => "ondalık sayı",
        LayerFieldKind::Date => "tarih",
        LayerFieldKind::DateTime => "tarih ve saat",
        LayerFieldKind::Time => "saat",
        LayerFieldKind::Boolean => "evet/hayır",
        LayerFieldKind::BigInt => "büyük tam sayı",
        LayerFieldKind::Instant => "zaman damgası",
        LayerFieldKind::Uuid => "UUID",
        LayerFieldKind::Json => "JSON",
        LayerFieldKind::Binary => "ikili",
    }
}
