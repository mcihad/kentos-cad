//! What every create command checks after its own input (docs/adr/0022,
//! 0027): the expected revision, then the layer; and how it writes. The
//! codes, paths and messages are the same for every command, so the shared
//! cases of each (`fixtures/commands/v1`) hold them the same way. The web's
//! counterpart is `apps/web/src/product/checks.ts`.

use std::collections::{BTreeMap, HashSet};

use kentos_commands::{
    CommandError, CommandResult, CommandWarning, Entity, LayerNodeType, MAX_LINE_WEIGHT,
    check_value,
};
use kentos_domain::{Document, Slot, Uuid};

use crate::codes;

/// Why a call stops before anything is written.
/// The error is boxed: a call stops at most once, and the `Result`s every
/// check returns stay small.
pub(crate) enum Stop {
    Failed(Box<CommandError>),
    Conflict(Box<CommandError>),
    /// The job it runs as was asked to stop (docs/adr/0253 §9): nothing written, `cancelled`.
    Cancelled,
}

impl Stop {
    pub(crate) fn failed(error: CommandError) -> Self {
        Stop::Failed(Box::new(error))
    }

    pub(crate) fn conflict(error: CommandError) -> Self {
        Stop::Conflict(Box::new(error))
    }

    /// The error it stops with, whichever way; a stop asked has none of its own.
    pub(crate) fn into_error(self) -> CommandError {
        match self {
            Stop::Failed(error) | Stop::Conflict(error) => *error,
            Stop::Cancelled => CommandError {
                code: "cancelled".into(),
                message: "İş durduruldu; hiçbir şey yazılmadı.".into(),
                ..CommandError::default()
            },
        }
    }
}

impl<T> From<Stop> for CommandResult<T> {
    fn from(stop: Stop) -> Self {
        match stop {
            Stop::Failed(error) => CommandResult::Failed { error: *error },
            Stop::Conflict(error) => CommandResult::Conflict { error: *error },
            Stop::Cancelled => CommandResult::Cancelled,
        }
    }
}

pub(crate) fn error(code: &str, message: String, path: Option<String>) -> CommandError {
    CommandError {
        code: code.to_owned(),
        message,
        path,
        ..CommandError::default()
    }
}

/// The line weight an input gives, when it gives one: a number from 0 to
/// 100 mm (`invalid_line_weight`, docs/adr/0139); NaN and ±∞ are not. `path`
/// names it. The message names no value: Rust and JavaScript write some
/// numbers differently (∞, 1e21), and both sides answer in the same words.
pub(crate) fn line_weight(weight: Option<f64>, path: &str) -> Result<(), Stop> {
    match weight {
        Some(w) if !(0.0..=MAX_LINE_WEIGHT).contains(&w) => Err(Stop::failed(error(
            codes::INVALID_LINE_WEIGHT,
            "Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı (0 en ince çizgidir). Bir kalınlık ya da “Katmana göre” seçin.".into(),
            Some(path.to_owned()),
        ))),
        _ => Ok(()),
    }
}

/// A new object's attributes on `layer` (docs/adr/0199 §2): the values given in their canonical
/// text by [`field_values`] (refused as it refuses), and the fields' defaults where none is given;
/// none when it has none. Every drawing command writes what this gives (FORM-O).
pub(crate) fn object_attrs(
    doc: &Document,
    layer: &str,
    given: Option<&BTreeMap<String, String>>,
    at: &str,
) -> Result<Option<BTreeMap<String, String>>, Stop> {
    let offered: BTreeMap<String, Option<String>> = given
        .into_iter()
        .flatten()
        .map(|(k, v)| (k.clone(), Some(v.clone())))
        .collect();
    let mut out: BTreeMap<String, String> = field_values(doc, layer, &offered, at)?
        .into_iter()
        .filter_map(|(k, v)| Some((k, v?)))
        .filter(|(_, v)| !v.is_empty())
        .collect();
    new_defaults(doc, layer, &mut out);
    subtype_fit(doc, layer, None, &out, at)?;
    combination_fit(doc, layer, None, &out, at)?;
    LayerRules::new(doc).fit(layer, None, &out, None, at)?;
    unique_fit(
        doc,
        &[UniqueWrite {
            slot: None,
            layer,
            before: None,
            after: &out,
            at: at.to_owned(),
        }],
    )?;
    Ok((!out.is_empty()).then_some(out))
}

/// The layers' record rules (Kayıt kuralları, docs/adr/0438 §1) asked of writes, each layer's
/// compiled once.
pub(crate) struct LayerRules<'d> {
    doc: &'d Document,
    compiled: std::collections::HashMap<String, kentos_forms::LayerRules>,
}

impl<'d> LayerRules<'d> {
    pub(crate) fn new(doc: &'d Document) -> LayerRules<'d> {
        LayerRules {
            doc,
            compiled: std::collections::HashMap::new(),
        }
    }

    /// A write of a record on `layer` from `before` (none: a new record) to `after` against the
    /// layer's rules: a rule it breaks is `rule_broken` at `{at}.{name}` of the rule's field (`at`
    /// without one); a broken warning rule is the warning `rule_unmet`. `object`: what the rules see of
    /// the record's geometry (none: its measures read empty, and a rule reading them holds).
    pub(crate) fn fit(
        &mut self,
        layer: &str,
        before: Option<&BTreeMap<String, String>>,
        after: &BTreeMap<String, String>,
        object: Option<&kentos_forms::Object>,
        at: &str,
    ) -> Result<Vec<CommandWarning>, Stop> {
        let Some(node) = self.doc.layers().get(layer).filter(|n| !n.rules.is_empty()) else {
            return Ok(Vec::new());
        };
        let doc = self.doc;
        let rules = self.compiled.entry(layer.to_owned()).or_insert_with(|| {
            kentos_forms::LayerRules::new(
                &node.rules,
                &node.fields,
                &crate::forms::variables_all(doc),
                crate::expression_unit::reading_unit(doc.settings()),
            )
            .unwrap_or_default()
        });
        let plain = kentos_forms::Object {
            layer: node.name.clone(),
            ..kentos_forms::Object::default()
        };
        let mut warnings = Vec::new();
        for broken in rules.broken(&node.fields, before, after, object.unwrap_or(&plain)) {
            let path = broken
                .field
                .map_or_else(|| at.to_owned(), |f| format!("{at}.{}", f.name));
            if !broken.rule.warning {
                return Err(Stop::failed(error(
                    codes::RULE_BROKEN,
                    format!("“{}” kuralı: {}", broken.rule.name, broken.rule.message),
                    Some(path),
                )));
            }
            warnings.push(CommandWarning {
                code: codes::RULE_UNMET.into(),
                message: format!("“{}” kuralı: {}", broken.rule.name, broken.rule.message),
                path: Some(path),
            });
        }
        Ok(warnings)
    }
}

/// A write a layer's unique fields are asked of (docs/adr/0438 §3).
pub(crate) struct UniqueWrite<'a> {
    /// The record written; none: a new one.
    pub slot: Option<Slot>,
    pub layer: &'a str,
    pub before: Option<&'a BTreeMap<String, String>>,
    pub after: &'a BTreeMap<String, String>,
    /// Where its values are (`attrs`, `objects[2].attrs`).
    pub at: String,
}

/// Writes against their layers' unique fields (docs/adr/0438 §3): a value written anew that another
/// record of the layer holds after the writes, the written ones among them, is `not_unique` at
/// `{at}.{name}`. A value a record kept is not asked; empty ones are not unique's.
pub(crate) fn unique_fit(doc: &Document, writes: &[UniqueWrite<'_>]) -> Result<(), Stop> {
    let mut layers: Vec<&str> = writes.iter().map(|w| w.layer).collect();
    layers.dedup();
    for layer in layers {
        let Some(node) = doc.layers().get(layer) else {
            continue;
        };
        let mine: Vec<&UniqueWrite<'_>> = writes.iter().filter(|w| w.layer == layer).collect();
        let written: HashSet<Slot> = mine.iter().filter_map(|w| w.slot).collect();
        for f in node.fields.iter().filter(|f| f.unique) {
            let fresh = |w: &UniqueWrite<'_>| -> Option<String> {
                let v = w.after.get(&f.name).filter(|v| !v.trim().is_empty())?;
                (w.before.and_then(|b| b.get(&f.name)) != Some(v)).then(|| v.clone())
            };
            if !mine.iter().any(|w| fresh(w).is_some()) {
                continue;
            }
            let mut counts: std::collections::HashMap<&str, usize> =
                std::collections::HashMap::new();
            for e in doc.by_layer(layer) {
                if written.contains(&Slot(e.base().id)) {
                    continue;
                }
                if let Some(v) = e.base().attrs.get(&f.name) {
                    *counts.entry(v.as_str()).or_default() += 1;
                }
            }
            for w in &mine {
                if let Some(v) = w.after.get(&f.name).filter(|v| !v.trim().is_empty()) {
                    *counts.entry(v.as_str()).or_default() += 1;
                }
            }
            for w in &mine {
                if let Some(v) = fresh(w)
                    && counts.get(v.as_str()).copied().unwrap_or(0) > 1
                {
                    return Err(Stop::failed(error(
                        codes::NOT_UNIQUE,
                        format!(
                            "“{}” alanı katmanda tekil; “{}” başka bir kayıtta var. Başka bir değer verin.",
                            f.label(),
                            kentos_commands::display_value(f, &v)
                        ),
                        Some(format!("{}.{}", w.at, f.name)),
                    )));
                }
            }
        }
    }
    Ok(())
}

/// A new record's empty fields filled (docs/adr/0199 §2, 0422 §2, 0752), as its form fills them: a form the layer
/// enforces gives its plain defaults first (so the subtype they choose is known), then the subtype field by its own
/// default, every other field by its subtype's default, else the field's. A field the form defaults by an expression
/// is left to the form, which works it out in the same step and gives the field's default when it gives none.
pub(crate) fn new_defaults(doc: &Document, layer: &str, out: &mut BTreeMap<String, String>) {
    let Some(node) = doc.layers().get(layer) else {
        return;
    };
    let (plain, by_expression) = doc.form_defaults(layer);
    for (key, d) in &plain {
        let Some(f) = node.fields.iter().find(|f| f.key() == key) else {
            continue;
        };
        if out
            .get(&f.name)
            .is_some_and(|v| !kentos_commands::js_trim(v).is_empty())
        {
            continue;
        }
        let loose = kentos_commands::LayerField {
            required: false,
            ..f.clone()
        };
        if let Ok(v) = kentos_commands::check_value(&loose, d)
            && !v.is_empty()
        {
            out.insert(f.name.clone(), v);
        }
    }
    kentos_commands::new_record_defaults(&node.fields, node.subtypes.as_ref(), out, |f| {
        by_expression.contains(f.key())
    });
}

/// A record's values on `layer` against the layer's value groups (docs/adr/0423 §2): a group whose
/// fields the write changed (any it fills, for a new record) and whose rows do not allow them is
/// `invalid_combination` when restrictive, at `{at}.{name}` of its first changed field; an advisory
/// one is the warning `combination_unmet`. A group the write did not touch is not asked.
pub(crate) fn combination_fit(
    doc: &Document,
    layer: &str,
    before: Option<&BTreeMap<String, String>>,
    after: &BTreeMap<String, String>,
    at: &str,
) -> Result<Vec<CommandWarning>, Stop> {
    let Some(node) = doc
        .layers()
        .get(layer)
        .filter(|n| !n.contingents.is_empty())
    else {
        return Ok(Vec::new());
    };
    let subtype = node
        .subtypes
        .as_ref()
        .and_then(|s| s.of(&node.fields, |n| after.get(n).cloned()))
        .map(|t| t.code.as_str());
    let mut warnings = Vec::new();
    for u in kentos_commands::unmet(&node.contingents, &node.fields, subtype, |n| {
        after.get(n).cloned()
    }) {
        let changed = u.group.fields.iter().find_map(|k| {
            let f = node.fields.iter().find(|f| f.key() == k)?;
            let now = after.get(&f.name);
            let was = before.and_then(|b| b.get(&f.name));
            (now != was && (before.is_some() || now.is_some())).then_some(f)
        });
        let Some(first) = changed else { continue };
        let words = kentos_commands::combination_words(u.group, &node.fields, &u.values);
        if u.group.restrictive {
            return Err(Stop::failed(error(
                codes::INVALID_COMBINATION,
                format!(
                    "“{}” değer grubunda bu bileşim yok: {words}. Grubun bileşimlerinden birini seçin.",
                    u.group.name
                ),
                Some(format!("{at}.{}", first.name)),
            )));
        }
        warnings.push(CommandWarning {
            code: codes::COMBINATION_UNMET.into(),
            message: format!("“{}” değer grubunda bu bileşim yok: {words}.", u.group.name),
            path: Some(format!("{at}.{}", first.name)),
        });
    }
    Ok(warnings)
}

/// A record's values on `layer` against its subtype (docs/adr/0422 §2): a value written anew (one that
/// differs from `before`) that the subtype does not offer is `invalid_attribute`, a required one of the
/// subtype written empty `attribute_required`, at `{at}.{name}`; values the record kept that its
/// subtype no longer allows, or required ones it left empty, are the warning `subtype_misfit`.
pub(crate) fn subtype_fit(
    doc: &Document,
    layer: &str,
    before: Option<&BTreeMap<String, String>>,
    after: &BTreeMap<String, String>,
    at: &str,
) -> Result<Option<CommandWarning>, Stop> {
    let Some(node) = doc.layers().get(layer) else {
        return Ok(None);
    };
    let Some(subtypes) = node.subtypes.as_ref() else {
        return Ok(None);
    };
    let Some(subtype) = subtypes.of(&node.fields, |n| after.get(n).cloned()) else {
        return Ok(None);
    };
    let words = subtypes.label_of(&node.fields, subtype);
    let anew = |name: &str| before.is_none_or(|b| b.get(name) != after.get(name));
    let mut kept: Vec<String> = Vec::new();
    for (f, codes) in
        kentos_commands::subtype_misfits(&node.fields, Some(subtype), |n| after.get(n).cloned())
    {
        if anew(&f.name) {
            return Err(Stop::failed(error(
                codes::INVALID_ATTRIBUTE,
                format!(
                    "“{}” alanına “{}”, “{words}” alt türünde verilemez: bu alt tür bu değeri sunmaz. Alt türün değerlerinden birini seçin.",
                    f.label(),
                    codes.join(", ")
                ),
                Some(format!("{at}.{}", f.name)),
            )));
        }
        kept.push(f.label().to_owned());
    }
    for f in &node.fields {
        let seen = kentos_commands::LayerSubtypes::field_for(f, Some(subtype));
        let empty = after.get(&f.name).is_none_or(|v| v.trim().is_empty());
        if seen.required && !f.required && empty {
            if before.is_some() && anew(&f.name) {
                return Err(Stop::failed(error(
                    codes::ATTRIBUTE_REQUIRED,
                    format!(
                        "“{}” alanı “{words}” alt türünde zorunlu; boş bırakılamaz.",
                        f.label()
                    ),
                    Some(format!("{at}.{}", f.name)),
                )));
            }
            if before.is_some() {
                kept.push(f.label().to_owned());
            }
        }
    }
    if kept.is_empty() {
        return Ok(None);
    }
    Ok(Some(CommandWarning {
        code: codes::SUBTYPE_MISFIT.into(),
        message: format!(
            "“{words}” alt türüne uymayan değerler kaldı: {}. Kaydı formla açıp düzeltin.",
            kept.join(", ")
        ),
        path: Some(at.to_owned()),
    }))
}

/// A record's values on `layer` against the fields its bound table always makes itself (docs/adr/0754 §3): a kept
/// record (`before`) whose value of such a field changes is `attribute_generated` at `{at}.{name}`. A new record's value
/// is the table's to replace when the record is written to it.
pub(crate) fn made_fit(
    doc: &Document,
    layer: &str,
    before: Option<&BTreeMap<String, String>>,
    after: &BTreeMap<String, String>,
    at: &str,
) -> Result<(), Stop> {
    let Some(before) = before else {
        return Ok(());
    };
    let Some(node) = doc.layers().get(layer) else {
        return Ok(());
    };
    for f in node.fields.iter().filter(|f| f.made_always()) {
        if before.get(&f.name) == after.get(&f.name) {
            continue;
        }
        let how = match f
            .column
            .as_ref()
            .and_then(kentos_commands::FieldColumn::making)
        {
            Some(kentos_commands::ColumnMade::Generated(Some(e))) => {
                format!("tablo onu hesaplıyor: {e}")
            }
            Some(kentos_commands::ColumnMade::Identity { .. }) => {
                "tablo onu her zaman kendisi numaralıyor (GENERATED ALWAYS AS IDENTITY)".to_owned()
            }
            _ => "tablo onu kendisi hesaplıyor (GENERATED ALWAYS)".to_owned(),
        };
        return Err(Stop::failed(error(
            codes::ATTRIBUTE_GENERATED,
            format!(
                "“{}” alanının değeri değiştirilemez: {how}. Değer, kayıt tabloya yazılınca tablodan gelir.",
                f.label()
            ),
            Some(format!("{at}.{}", f.name)),
        )));
    }
    Ok(())
}

/// The attributes written to an object on `layer`, by its fields
/// (docs/adr/0199 §2): each value given to a field's key in its canonical
/// text, a null (taken away) checked as empty; in the names' order, the first
/// that does not keep its field's rules `invalid_attribute`, an empty value of
/// a required field `attribute_required`, at `{at}.{name}`. Keys no field
/// names, and every key on a layer without fields, stay as given.
pub(crate) fn field_values(
    doc: &Document,
    layer: &str,
    attrs: &BTreeMap<String, Option<String>>,
    at: &str,
) -> Result<BTreeMap<String, Option<String>>, Stop> {
    let Some(node) = doc.layers().get(layer).filter(|n| !n.fields.is_empty()) else {
        return Ok(attrs.clone());
    };
    let mut out = BTreeMap::new();
    for (key, value) in attrs {
        let Some(field) = node.fields.iter().find(|f| f.name == *key) else {
            out.insert(key.clone(), value.clone());
            continue;
        };
        match check_value(field, value.as_deref().unwrap_or_default()) {
            Ok(canonical) => {
                out.insert(key.clone(), value.as_ref().map(|_| canonical));
            }
            Err(refusal) => {
                let code = if refusal.code == "required" {
                    codes::ATTRIBUTE_REQUIRED
                } else {
                    codes::INVALID_ATTRIBUTE
                };
                return Err(Stop::failed(error(
                    code,
                    refusal.message,
                    Some(format!("{at}.{key}")),
                )));
            }
        }
    }
    Ok(out)
}

/// A value that is NaN or ±∞, as a message names it: “2. noktanın doğu (Y)”.
pub(crate) fn not_finite(whose: &str, axis: &str, path: String) -> Stop {
    let name = if axis == "x" {
        "doğu (Y)"
    } else {
        "kuzey (X)"
    };
    Stop::failed(error(
        codes::NOT_FINITE,
        format!(
            "{whose} {name} değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin."
        ),
        Some(path),
    ))
}

/// The first coordinate of `p` that is NaN or ±∞, x before y (`whose`: “Merkezin”).
pub(crate) fn point(p: kentos_commands::Vec2, whose: &str, path: &str) -> Result<(), Stop> {
    for (axis, value) in [("x", p.x), ("y", p.y)] {
        if !value.is_finite() {
            return Err(not_finite(whose, axis, format!("{path}.{axis}")));
        }
    }
    Ok(())
}

/// A number that is NaN or ±∞ (a radius, an angle, an elevation), as its
/// message names it: “Yarıçap sonlu bir sayı değil …”.
pub(crate) fn finite(value: f64, what: &str, fix: &str, path: &str) -> Result<(), Stop> {
    if value.is_finite() {
        return Ok(());
    }
    Err(Stop::failed(error(
        codes::NOT_FINITE,
        format!("{what} sonlu bir sayı değil (NaN ya da sonsuz). {fix}"),
        Some(path.into()),
    )))
}

/// A circle's or an arc's radius: above zero (`invalid_radius`), once it is finite.
pub(crate) fn radius(r: f64) -> Result<(), Stop> {
    if r > 0.0 {
        return Ok(());
    }
    Err(Stop::failed(error(
        codes::INVALID_RADIUS,
        "Yarıçap sıfırdan büyük olmalı. Pozitif bir yarıçap verin.".into(),
        Some("r".into()),
    )))
}

/// The expected revision, when given: decimal text, then the document's own
/// (`conflict` when not, with the revision now).
pub(crate) fn revision(doc: &Document, expected: Option<&str>) -> Result<(), Stop> {
    let Some(expected) = expected else {
        return Ok(());
    };
    if !is_revision_text(expected) {
        return Err(Stop::failed(error(
            codes::INVALID_REVISION,
            format!(
                "Beklenen sürüm “{expected}” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."
            ),
            Some("expectedRevision".into()),
        )));
    }
    let current = doc.revision().to_string();
    if expected != current {
        return Err(Stop::conflict(CommandError {
            revision: Some(current),
            ..error(
                codes::REVISION_CONFLICT,
                "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.".into(),
                Some("expectedRevision".into()),
            )
        }));
    }
    Ok(())
}

/// The layer the object goes on: known, a layer and not a group, not locked
/// (by itself or a group above it). A hidden one takes the object with a
/// warning. The locked and hidden texts are the drawing tools' own words
/// (web `tools/targetLayer.ts`), kept since the tools write through here.
pub(crate) fn layer(doc: &Document, id: &str) -> Result<Vec<CommandWarning>, Stop> {
    layer_at(doc, id, "layerId")
}

/// [`layer`], its refusals and warning at `path` (`changes[2].layerId`).
pub(crate) fn layer_at(doc: &Document, id: &str, path: &str) -> Result<Vec<CommandWarning>, Stop> {
    let layers = doc.layers();
    let at = || Some(path.to_owned());
    let Some(node) = layers.get(id) else {
        return Err(Stop::failed(error(
            codes::LAYER_NOT_FOUND,
            format!("“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."),
            at(),
        )));
    };
    let name = &node.name;
    if node.kind != LayerNodeType::Layer {
        return Err(Stop::failed(error(
            codes::NOT_A_LAYER,
            format!(
                "“{name}” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin."
            ),
            at(),
        )));
    }
    // A layer drawn from a map service holds no objects (docs/adr/0208 §2).
    if node.service.is_some() {
        return Err(Stop::failed(error(
            codes::SERVICE_LAYER,
            format!(
                "“{name}” bir servis katmanı; çizimi servisten gelir, ona nesne eklenmez. Başka bir katmanı etkinleştirin."
            ),
            at(),
        )));
    }
    if layers.is_locked(id) {
        return Err(Stop::failed(error(
            codes::LAYER_LOCKED,
            format!(
                "“{name}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin."
            ),
            at(),
        )));
    }
    let mut warnings = Vec::new();
    if !layers.is_visible(id) {
        warnings.push(CommandWarning {
            code: codes::LAYER_HIDDEN.into(),
            message: format!("“{name}” katmanı gizli; çizilen nesne görünmeyecek."),
            path: at(),
        });
    }
    Ok(warnings)
}

/// The ids a command names (`cad.entities.delete`, `cad.entities.transform`):
/// at least one (`no_entities`, with the command's own sentence `nothing`),
/// each lowercase UUID text with hyphens (`invalid_uid`, the first that is not).
pub(crate) fn uids(uids: &[String], nothing: &str) -> Result<(), Stop> {
    if uids.is_empty() {
        return Err(Stop::failed(error(
            codes::NO_ENTITIES,
            format!("{nothing} En az bir nesnenin kalıcı kimliğini verin."),
            Some("uids".into()),
        )));
    }
    for (i, uid) in uids.iter().enumerate() {
        if !is_uid_text(uid) {
            return Err(Stop::failed(error(
                codes::INVALID_UID,
                format!(
                    "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın."
                ),
                Some(format!("uids[{i}]")),
            )));
        }
    }
    Ok(())
}

/// The objects the ids name, each once, in the input's order: its slot, the
/// object and its id. `entity_not_found` for the first the document lacks.
pub(crate) fn objects<'a>(
    doc: &'a Document,
    uids: &'a [String],
) -> Result<Vec<(Slot, &'a Entity, &'a String)>, Stop> {
    Ok(named(doc, uids)?
        .into_iter()
        .map(|(_, slot, entity, uid)| (slot, entity, uid))
        .collect())
}

/// [`objects`], each with where the input first names it (`uids[at]`), for
/// a refusal about one of them.
pub(crate) fn named<'a>(
    doc: &'a Document,
    uids: &'a [String],
) -> Result<Vec<(usize, Slot, &'a Entity, &'a String)>, Stop> {
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(uids.len());
    for (i, uid) in uids.iter().enumerate() {
        if !seen.insert(uid.as_str()) {
            continue;
        }
        let found = Uuid::parse_str(uid)
            .ok()
            .and_then(|u| doc.slot_of(u))
            .and_then(|slot| Some((slot, doc.get(slot)?)));
        let Some((slot, entity)) = found else {
            return Err(Stop::failed(error(
                codes::ENTITY_NOT_FOUND,
                format!(
                    "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
                ),
                Some(format!("uids[{i}]")),
            )));
        };
        out.push((i, slot, entity, uid));
    }
    Ok(out)
}

/// An object a command names: where the input names it, its slot, itself and its id ([`named`]).
pub(crate) type Named<'a> = (usize, Slot, &'a Entity, &'a String);

/// The objects a command changes one by one, those on a locked layer (by itself or a group above it) left out
/// (docs/adr/0278): every one locked, refused with the first one's words at its place; else the others, and a
/// warning counting the left out.
pub(crate) fn skip_locked<'a>(
    doc: &Document,
    found: Vec<Named<'a>>,
) -> Result<(Vec<Named<'a>>, Option<CommandWarning>), Stop> {
    let layers = doc.layers();
    let (locked, open): (Vec<_>, Vec<_>) = found
        .into_iter()
        .partition(|(_, _, e, _)| layers.is_locked(&e.base().layer_id));
    if open.is_empty()
        && let Some((at, _, e, _)) = locked.first()
    {
        let layer = &e.base().layer_id;
        let name = layers
            .get(layer)
            .map_or(layer.as_str(), |n| n.name.as_str());
        return Err(Stop::failed(error(
            codes::LAYER_LOCKED,
            format!(
                "“{name}” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın."
            ),
            Some(format!("uids[{at}]")),
        )));
    }
    let warning = (!locked.is_empty()).then(|| CommandWarning {
        code: codes::LAYER_LOCKED.into(),
        message: format!(
            "{} nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın.",
            locked.len()
        ),
        path: Some("uids".into()),
    });
    Ok((open, warning))
}

/// A text that is empty or only white space, as Unicode's `White_Space` has
/// it (the web's `isBlank`; not JavaScript's `trim`, which takes U+FEFF and
/// leaves U+0085).
pub(crate) fn is_blank(text: &str) -> bool {
    text.chars().all(char::is_whitespace)
}

/// A persistent id as the contract writes it: lowercase hexadecimal with
/// hyphens, 8-4-4-4-12 (the web's `isUuid`).
pub(crate) fn is_uid_text(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => *b == b'-',
            _ => b.is_ascii_digit() || (b'a'..=b'f').contains(b),
        })
}

/// Writes one object as one undo step (“Ekle”, the document's own `add`),
/// into the open transaction or group if one is: its slot, persistent id and
/// the revision after, or `slots_exhausted` when the document has no slot left.
pub(crate) fn add(doc: &mut Document, entity: Entity) -> Result<(u32, String, String), Stop> {
    match doc.add(entity) {
        Ok(slot) => Ok(written(doc, slot)),
        Err(full) => Err(Stop::failed(error(
            codes::SLOTS_EXHAUSTED,
            full.to_string(),
            None,
        ))),
    }
}

fn written(doc: &Document, slot: Slot) -> (u32, String, String) {
    let uid = doc.uid(slot).map(|u| u.to_string()).unwrap_or_default();
    (slot.0, uid, doc.revision().to_string())
}

/// Decimal text of a whole number, no sign, no leading zero: `0`, `12`.
/// Any length: a revision the document never had is a conflict, not an error.
fn is_revision_text(text: &str) -> bool {
    let digits = text.bytes().all(|b| b.is_ascii_digit());
    digits && !text.is_empty() && (text == "0" || !text.starts_with('0'))
}
