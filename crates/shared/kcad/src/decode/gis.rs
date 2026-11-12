//! The GIS settings as a payload keeps them: forms, value lists, relations, bound sources and their bases, join
//! views, linear events, lineage and profile files, with their schemas' checks.

use super::*;

/// `forms`: what the payload's schema lets it hold, checked as it is read.
pub(super) fn forms(r: &mut Reader<'_>) -> Result<Vec<kentos_contracts::ProjectForm>, KcadError> {
    let at = r.position();
    let value = r.opaque()?;
    let list: Vec<kentos_contracts::ProjectForm> =
        serde_json::from_value(value.clone()).map_err(|e| {
            let words = e.to_string();
            let code = if words.contains("unknown field") {
                Code::UnknownField
            } else {
                Code::BadValue
            };
            r.fail_at(code, at, &format!("formlar okunamadı: {words}"))
        })?;
    // Read back exactly as the writer writes it: no field at its default.
    if serde_json::to_value(&list).ok().as_ref() != Some(&value) {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "formlar yazıcının biçiminde değil (varsayılan değerinde bir alan yazılmış)",
        ));
    }
    if list.is_empty() {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "boş liste yazılmaz; form yoksa alan hiç yazılmaz",
        ));
    }
    if let Some(problem) = kentos_contracts::forms_problem(&list) {
        return Err(r.fail_at(Code::BadValue, at, &problem));
    }
    Ok(list)
}

/// `domains`: what the payload's schema lets it hold, checked as it is read.
pub(super) fn domains(r: &mut Reader<'_>) -> Result<Vec<kentos_contracts::FieldDomain>, KcadError> {
    let at = r.position();
    let value = r.opaque()?;
    let list: Vec<kentos_contracts::FieldDomain> =
        serde_json::from_value(value.clone()).map_err(|e| {
            let words = e.to_string();
            let code = if words.contains("unknown field") {
                Code::UnknownField
            } else {
                Code::BadValue
            };
            r.fail_at(code, at, &format!("değer listeleri okunamadı: {words}"))
        })?;
    if serde_json::to_value(&list).ok().as_ref() != Some(&value) {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "değer listeleri yazıcının biçiminde değil (varsayılan değerinde bir alan yazılmış)",
        ));
    }
    if list.is_empty() {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "boş liste yazılmaz; değer listesi yoksa alan hiç yazılmaz",
        ));
    }
    if let Some((_, problem)) = kentos_contracts::domains_problem(&list) {
        return Err(r.fail_at(Code::BadValue, at, &problem));
    }
    Ok(list)
}

/// `sources`: what the payload's schema lets it hold, checked as it is read.
pub(super) fn sources(
    r: &mut Reader<'_>,
    has: Features,
) -> Result<Vec<kentos_contracts::SourceLink>, KcadError> {
    let at = r.position();
    let value = r.opaque()?;
    let list: Vec<kentos_contracts::SourceLink> = serde_json::from_value(value.clone())
        .map_err(|e| {
            let words = e.to_string();
            let code = if words.contains("unknown field") {
                Code::UnknownField
            } else {
                Code::BadValue
            };
            r.fail_at(code, at, &format!("kaynaklar okunamadı: {words}"))
        })?;
    if serde_json::to_value(&list).ok().as_ref() != Some(&value) {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "kaynaklar yazıcının biçiminde değil (varsayılan değerinde bir alan yazılmış)",
        ));
    }
    if list.is_empty() {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "boş liste yazılmaz; bağlı katman yoksa alan hiç yazılmaz",
        ));
    }
    // Schema 134 (docs/adr/0755 §1): a link's CHECK and unique constraints.
    if !has.table_constraints
        && list
            .iter()
            .any(|s| !s.checks.is_empty() || !s.uniques.is_empty())
    {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "kaynakların checks ve uniques'i şema 134'ten (docs/adr/0755)",
        ));
    }
    if let Some(problem) = kentos_contracts::sources_problem(&list, &[]) {
        return Err(r.fail_at(Code::BadValue, at, &problem));
    }
    Ok(list)
}

/// `sourceBases`: what the payload's schema lets it hold, checked as it is read.
pub(super) fn source_bases(
    r: &mut Reader<'_>,
    has: Features,
) -> Result<Vec<kentos_contracts::SourceBase>, KcadError> {
    let at = r.position();
    let value = r.opaque()?;
    let list: Vec<kentos_contracts::SourceBase> = serde_json::from_value(value.clone())
        .map_err(|e| {
            let words = e.to_string();
            let code = if words.contains("unknown field") {
                Code::UnknownField
            } else {
                Code::BadValue
            };
            r.fail_at(code, at, &format!("kaynak tabanları okunamadı: {words}"))
        })?;
    if list.is_empty() {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "boş liste yazılmaz; taban yoksa alan hiç yazılmaz",
        ));
    }
    // Schema 92 (docs/adr/0553 §1): the field records are unknown before it.
    if !has.source_field_bases && list.iter().any(|b| b.fields.is_some()) {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "kaynak tabanının alan kayıtları şema 92'den önce yok",
        ));
    }
    Ok(list)
}

/// `relations`: what the payload's schema lets it hold, checked as it is read.
pub(super) fn relations(
    r: &mut Reader<'_>,
) -> Result<Vec<kentos_contracts::ProjectRelation>, KcadError> {
    let at = r.position();
    let value = r.opaque()?;
    let list: Vec<kentos_contracts::ProjectRelation> =
        serde_json::from_value(value.clone()).map_err(|e| {
            let words = e.to_string();
            let code = if words.contains("unknown field") {
                Code::UnknownField
            } else {
                Code::BadValue
            };
            r.fail_at(code, at, &format!("ilişkiler okunamadı: {words}"))
        })?;
    if serde_json::to_value(&list).ok().as_ref() != Some(&value) {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "ilişkiler yazıcının biçiminde değil (varsayılan değerinde bir alan yazılmış)",
        ));
    }
    if list.is_empty() {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "boş liste yazılmaz; ilişki yoksa alan hiç yazılmaz",
        ));
    }
    if let Some(problem) = kentos_contracts::relations_problem(&list) {
        return Err(r.fail_at(Code::BadValue, at, &problem));
    }
    Ok(list)
}

/// `joinViews`: what the payload's schema lets it hold, checked as it is read.
pub(super) fn join_views(
    r: &mut Reader<'_>,
    has: Features,
) -> Result<Vec<kentos_contracts::JoinView>, KcadError> {
    let at = r.position();
    let value = r.opaque()?;
    let list: Vec<kentos_contracts::JoinView> = serde_json::from_value(value.clone())
        .map_err(|e| {
        let words = e.to_string();
        let code = if words.contains("unknown field") {
            Code::UnknownField
        } else {
            Code::BadValue
        };
        r.fail_at(code, at, &format!("birleşik görünümler okunamadı: {words}"))
    })?;
    if serde_json::to_value(&list).ok().as_ref() != Some(&value) {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "birleşik görünümler yazıcının biçiminde değil (varsayılan değerinde bir alan yazılmış)",
        ));
    }
    if list.is_empty() {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "boş liste yazılmaz; görünüm yoksa alan hiç yazılmaz",
        ));
    }
    // Schema 105 (docs/adr/0632 §4): a view's source table and summing columns.
    if !has.join_sources
        && list
            .iter()
            .any(|v| v.source.is_some() || !v.aggregates.is_empty())
    {
        return Err(r.fail_at(
            Code::UnknownField,
            at,
            "birleşik görünümün source ve aggregates alanları şema 105'indir",
        ));
    }
    if let Some(problem) = kentos_contracts::join_views_problem(&list) {
        return Err(r.fail_at(Code::BadValue, at, &problem));
    }
    Ok(list)
}

/// `linearEvents`: what the payload's schema lets it hold, checked as it is read.
pub(super) fn linear_events(
    r: &mut Reader<'_>,
) -> Result<Vec<kentos_contracts::EventLink>, KcadError> {
    let at = r.position();
    let value = r.opaque()?;
    let list: Vec<kentos_contracts::EventLink> = serde_json::from_value(value.clone())
        .map_err(|e| {
            let words = e.to_string();
            let code = if words.contains("unknown field") {
                Code::UnknownField
            } else {
                Code::BadValue
            };
            r.fail_at(code, at, &format!("olay bağları okunamadı: {words}"))
        })?;
    if serde_json::to_value(&list).ok().as_ref() != Some(&value) {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "olay bağları yazıcının biçiminde değil (varsayılan değerinde bir alan yazılmış)",
        ));
    }
    if list.is_empty() {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "boş liste yazılmaz; bağ yoksa alan hiç yazılmaz",
        ));
    }
    if let Some(problem) = kentos_contracts::event_links_problem(&list) {
        return Err(r.fail_at(Code::BadValue, at, &problem));
    }
    Ok(list)
}

/// `profileFiles`: what the payload's schema lets it hold, checked as it is read.
pub(super) fn profile_files(
    r: &mut Reader<'_>,
) -> Result<Vec<kentos_contracts::ProjectProfileFile>, KcadError> {
    let at = r.position();
    let value = r.opaque()?;
    let list: Vec<kentos_contracts::ProjectProfileFile> =
        serde_json::from_value(value.clone()).map_err(|e| {
            let words = e.to_string();
            let code = if words.contains("unknown field") {
                Code::UnknownField
            } else {
                Code::BadValue
            };
            r.fail_at(code, at, &format!("profil dosyaları okunamadı: {words}"))
        })?;
    if list.is_empty() {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "boş liste yazılmaz; profil yoksa alan hiç yazılmaz",
        ));
    }
    if let Some(problem) = kentos_contracts::profile_files_problem(&list) {
        return Err(r.fail_at(Code::BadValue, at, &problem));
    }
    Ok(list)
}

/// `lineage`: what the payload's schema lets it hold, checked as it is read.
pub(super) fn lineage(
    r: &mut Reader<'_>,
) -> Result<Vec<kentos_contracts::LineageRecord>, KcadError> {
    let at = r.position();
    let value = r.opaque()?;
    let list: Vec<kentos_contracts::LineageRecord> =
        serde_json::from_value(value.clone()).map_err(|e| {
            let words = e.to_string();
            let code = if words.contains("unknown field") {
                Code::UnknownField
            } else {
                Code::BadValue
            };
            r.fail_at(code, at, &format!("soy kayıtları okunamadı: {words}"))
        })?;
    if serde_json::to_value(&list).ok().as_ref() != Some(&value) {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "soy kayıtları yazıcının biçiminde değil",
        ));
    }
    if list.is_empty() {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "boş liste yazılmaz; kayıt yoksa alan hiç yazılmaz",
        ));
    }
    if let Some(problem) = kentos_contracts::lineage_problem(&list) {
        return Err(r.fail_at(Code::BadValue, at, &problem));
    }
    Ok(list)
}

/// Schema 27's topology settings (docs/adr/0202 §7): the tolerance, the
/// rules (`id`, `kind`, `layer`, `other`, `value`) and the exceptions (`at`,
/// `rule`, `objects`; from schema 99 `digest`; from schema 189 `decision`, `reason`, `ruleDigest`, `scope`) and, from
/// schema 189, the quality `profile` (`name`, `edition`, `precision`, `acceptance`, docs/adr/1260), checked whole as the
/// contract checks them (`TopologySettings::problem`).
pub(super) fn topology_settings(
    r: &mut Reader<'_>,
    digests: bool,
    reviews: bool,
) -> Result<TopologySettings, KcadError> {
    let at = r.position();
    let mut t = TopologySettings::default();
    map(r, |r, key| {
        match key {
            "rules" => {
                t.rules = list(r, |r, _| {
                    let (mut id, mut kind, mut layer, mut other, mut value) =
                        (None, None, None, None, None);
                    map(r, |r, key| {
                        match key {
                            "id" => id = Some(text(r)?),
                            "kind" => {
                                let at = r.position();
                                let k = r.text()?;
                                kind = Some(TopologyRuleKind::of_key(k).ok_or_else(|| {
                                    r.fail_at(
                                        Code::BadValue,
                                        at,
                                        &format!("“{k}” bilinen bir topoloji kuralı değil"),
                                    )
                                })?);
                            }
                            "layer" => layer = Some(text(r)?),
                            "other" => other = Some(text(r)?),
                            "value" => value = Some(r.float()?),
                            _ => return Err(unknown(r)),
                        }
                        Ok(())
                    })?;
                    Ok(TopologyRule {
                        id: required(r, id, "id")?,
                        kind: required(r, kind, "kind")?,
                        layer: required(r, layer, "layer")?,
                        other,
                        value,
                    })
                })?
            }
            "tolerance" => t.tolerance = Some(r.float()?),
            "profile" if reviews => {
                let (mut name, mut edition, mut precision, mut acceptance) = (None, None, None, None);
                map(r, |r, key| {
                    match key {
                        "name" => name = Some(text(r)?),
                        "edition" => edition = Some(text(r)?),
                        "precision" => precision = Some(r.float()?),
                        "acceptance" => acceptance = Some(r.float()?),
                        _ => return Err(unknown(r)),
                    }
                    Ok(())
                })?;
                t.profile = Some(TopologyProfile {
                    name: required(r, name, "name")?,
                    edition,
                    precision: required(r, precision, "precision")?,
                    acceptance: required(r, acceptance, "acceptance")?,
                });
            }
            "exceptions" => {
                t.exceptions = list(r, |r, _| {
                    let (mut at, mut rule, mut objects, mut digest) = (None, None, None, None);
                    let (mut decision, mut reason, mut rule_digest, mut scope) = (None, None, None, None);
                    map(r, |r, key| {
                        match key {
                            "at" => at = Some(point(r)?),
                            "rule" => rule = Some(text(r)?),
                            "objects" => {
                                objects =
                                    Some(list(r, |r, _| id16(r).map(kentos_contracts::EntityId))?)
                            }
                            "digest" if digests => digest = Some(text(r)?),
                            "decision" if reviews => {
                                let at = r.position();
                                decision = Some(match r.text()? {
                                    "exception" => TopologyDecision::Exception,
                                    "error" => TopologyDecision::Error,
                                    k => {
                                        return Err(r.fail_at(
                                            Code::BadValue,
                                            at,
                                            &format!("“{k}” bilinen bir topoloji kararı değil"),
                                        ));
                                    }
                                });
                            }
                            "reason" if reviews => reason = Some(text(r)?),
                            "ruleDigest" if reviews => rule_digest = Some(text(r)?),
                            "scope" if reviews => {
                                let at = r.position();
                                scope = Some(match r.text()? {
                                    "finding" => TopologyScope::Finding,
                                    "objects" => TopologyScope::Objects,
                                    k => {
                                        return Err(r.fail_at(
                                            Code::BadValue,
                                            at,
                                            &format!("“{k}” bilinen bir istisna kapsamı değil"),
                                        ));
                                    }
                                });
                            }
                            _ => return Err(unknown(r)),
                        }
                        Ok(())
                    })?;
                    Ok(TopologyException {
                        rule: required(r, rule, "rule")?,
                        objects: required(r, objects, "objects")?,
                        at: required(r, at, "at")?,
                        digest,
                        decision,
                        reason,
                        rule_digest,
                        scope,
                    })
                })?
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    match t.problem() {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(t),
    }
}
