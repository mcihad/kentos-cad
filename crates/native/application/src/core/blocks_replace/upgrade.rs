//! Explicit publication upgrade: isolate the incoming graph, then reuse ordinary replacement checks.
use std::collections::{HashMap, HashSet};

use kentos_commands::{BlockDefinition, BlockId, BlocksReplace, Entity};
use kentos_domain::Document;
use sha2::{Digest, Sha256};

use crate::{
    checks::{self, Stop, error},
    codes,
};

fn refused(message: impl Into<String>) -> Stop {
    Stop::failed(error(
        codes::INVALID_LIBRARY,
        message.into(),
        Some("library".into()),
    ))
}

pub(super) fn check(
    doc: &Document,
    input: &BlocksReplace,
    text: &str,
) -> Result<super::Checked, Stop> {
    let (Some(from), Some(to)) = (input.from, input.to) else {
        return Err(Stop::failed(error(
            codes::NO_BLOCK,
            "Yükseltilecek blok ve kitaplıktaki hedef blok kimliği gerekli.".into(),
            Some(if input.from.is_none() { "from" } else { "to" }.into()),
        )));
    };
    checks::revision(doc, input.expected_revision.as_deref())?;
    let old = doc
        .block(from)
        .ok_or_else(|| super::unknown(from, "from"))?;
    if input.purge == Some(true) {
        return Err(refused(
            "Yayın yükseltmesinde eski tanım korunur; purge kullanılamaz.",
        ));
    }
    let file: serde_json::Value = serde_json::from_str(text)
        .map_err(|_| refused("Yayın kaynağı geçerli bir kentos.blocks JSON dosyası olmalı."))?;
    if file["format"].as_str() != Some(kentos_commands::BLOCKS_FORMAT)
        || !matches!(file["version"].as_u64(), Some(v) if v == u64::from(kentos_commands::BLOCKS_PUBLICATION_VERSION) || v == u64::from(kentos_commands::BLOCKS_TYPE_VALUES_VERSION))
    {
        return Err(refused(
            "Yükseltme, yayın kimlikli kentos.blocks sürüm 2 veya 3 gerektirir.",
        ));
    }
    let blocks: Vec<BlockDefinition> = serde_json::from_value(file["blocks"].clone())
        .map_err(|e| refused(format!("Yayın tanımları okunamadı: {e}")))?;
    if file["version"].as_u64().unwrap_or_default() < u64::from(kentos_commands::block_library_version(&blocks)) {
        return Err(refused("Ortak tip değeri içeren yayın kaynağı kentos.blocks sürüm 3 olmalı."));
    }
    crate::blocks_define::block_rules(&blocks.iter().collect::<Vec<_>>())
        .map_err(|why| refused(why.into_error().message))?;
    let source = blocks
        .iter()
        .find(|b| b.id == to)
        .ok_or_else(|| refused("Seçilen hedef blok kitaplıkta bulunamadı."))?;
    let (Some(a), Some(b)) = (
        old.card.as_ref().and_then(|c| c.publication.as_ref()),
        source.card.as_ref().and_then(|c| c.publication.as_ref()),
    ) else {
        return Err(refused(
            "Her iki blokta aile, tip ve yayın kimliği olmalı. Normal blok değiştirme ayrı kullanılabilir.",
        ));
    };
    if a.family_id != b.family_id || a.type_id != b.type_id {
        return Err(refused(
            "Yayın yükseltmesi aynı aile ve aynı tip kimliğini gerektirir.",
        ));
    }
    if b.release <= a.release {
        return Err(refused(format!(
            "Hedef yayın {} mevcut {} numarasından yeni değil. Çizim korunuyor.",
            b.release, a.release
        )));
    }
    let release = b.release;
    if (old.units == Some(kentos_commands::BlockUnits::Unitless))
        != (source.units == Some(kentos_commands::BlockUnits::Unitless))
    {
        return Err(refused(
            "Birimsiz ve fiziksel yayınlar doğrudan yükseltilemez. Önce kaynak bileşenin birimini açıkça düzeltin; yerleştirmeler değiştirilmedi.",
        ));
    }
    // Only the target's complete dependency closure is copied; no existing parent/child is redefined by name.
    let by_id: HashMap<_, _> = blocks.iter().map(|b| (b.id, b)).collect();
    let mut needed = HashSet::new();
    let mut open = vec![to];
    while let Some(id) = open.pop() {
        if !needed.insert(id) {
            continue;
        }
        let definition = by_id[&id];
        if definition.xref.is_some() {
            return Err(refused(
                "Harici referans içeren yayın bu yolla yükseltilemez; önce kaynağı bağımsız bileşen yapın.",
            ));
        }
        open.extend(definition.entities.iter().filter_map(|e| match e {
            Entity::Insert(i) => Some(i.block),
            _ => None,
        }));
    }
    let mut graph: Vec<_> = blocks
        .iter()
        .filter(|b| needed.contains(&b.id))
        .cloned()
        .collect();
    graph.sort_by_key(|b| b.id.0);
    // Stable ids make guarded plan/apply and a later selected scope refer to precisely the same graph.
    let ids: HashMap<_, _> = graph
        .iter()
        .map(|definition| {
            let mut closure = HashSet::new();
            let mut pending = vec![definition.id];
            while let Some(id) = pending.pop() {
                if !closure.insert(id) {
                    continue;
                }
                pending.extend(by_id[&id].entities.iter().filter_map(|e| match e {
                    Entity::Insert(i) => Some(i.block),
                    _ => None,
                }));
            }
            let mut content: Vec<_> = closure.into_iter().map(|id| by_id[&id]).collect();
            content.sort_by_key(|b| b.id.0);
            let value = serde_json::to_value(content)
                .map_err(|why| refused(format!("Yayın içeriği hazırlanamadı: {why}")))?;
            let content = kentos_commands::canonical::canonical_json(&value);
            let mut hash = Sha256::new();
            hash.update(b"kentos.block-publication-upgrade/1\0");
            hash.update(content.as_bytes());
            hash.update(definition.id.0);
            let mut bytes = [0; 16];
            bytes.copy_from_slice(&hash.finalize()[..16]);
            bytes[6] = (bytes[6] & 15) | 128;
            bytes[8] = (bytes[8] & 63) | 128;
            Ok((definition.id, BlockId(bytes)))
        })
        .collect::<Result<_, Stop>>()?;
    let target = ids[&to];
    let mut names: HashSet<_> = doc
        .blocks()
        .iter()
        .map(|b| kentos_commands::blocks::name_key(&b.name))
        .collect();
    let mut added = Vec::new();
    for mut definition in graph {
        definition.id = ids[&definition.id];
        for e in &mut definition.entities {
            if let Entity::Insert(i) = e {
                i.block = ids[&i.block];
            }
        }
        if let Some(existing) = doc.block(definition.id) {
            definition.name = existing.name.clone();
            if existing != &definition {
                return Err(refused(
                    "Bu yayının çizimdeki tanımı yerel olarak değiştirilmiş. Yeni kapsamı uygulamadan önce farklı bir yayın hazırlayın; yerel tanım ezilmedi.",
                ));
            }
            continue;
        }
        if definition
            .card
            .as_ref()
            .and_then(|c| c.publication.as_ref())
            .is_some_and(|p| {
                doc.blocks().iter().any(|have| {
                    have.card
                        .as_ref()
                        .and_then(|c| c.publication.as_ref())
                        .is_some_and(|q| {
                            p.family_id == q.family_id
                                && p.type_id == q.type_id
                                && p.release == q.release
                        })
                })
            })
        {
            return Err(refused(
                "Aynı yayın kimliği çizimde farklı bir kaynak tanımıyla var. Yeni yayın numarası hazırlayın; mevcut tanım ezilmedi.",
            ));
        }
        let original = definition.name.clone();
        let mut n = 1;
        while !names.insert(kentos_commands::blocks::name_key(&definition.name)) {
            definition.name = if n == 1 {
                format!("{original} · Yayın {release}")
            } else {
                format!("{original} · Yayın {release} ({n})")
            };
            n += 1;
        }
        added.push(definition);
    }
    let mut copy = doc.clone();
    copy.merge_blocks(&added)
        .map_err(|why| refused(why.to_string()))?;
    let mut replacement = input.clone();
    replacement.library = None;
    replacement.to = Some(target);
    let mut result = super::check(&copy, &replacement)?;
    if !result.changes.is_empty() {
        result.output.target = Some(target);
        result.output.added = added.iter().map(|b| b.id).collect();
        result.added = added;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ExecutionContext;
    use kentos_commands::{CallEnvelope, CallExpectations, CallOp, CommandResult};
    use serde_json::json;

    fn setup() -> (Document, BlocksReplace) {
        let snapshot = kentos_kcad::decode(include_bytes!(
            "../../../../../../fixtures/kcad/v2/block-publication.kcad"
        ))
        .unwrap();
        let from = snapshot.blocks[0].id;
        let uid = snapshot.uids[1].to_text();
        let mut doc = Document::from_snapshot_v2(snapshot).unwrap();
        let mut child = doc.block(from).unwrap().clone();
        child.id = BlockId([9; 16]);
        child.name = "Donatı".into();
        child.dynamic = None;
        child.card = None;
        child.entities.truncate(1);
        doc.add_block(child.clone()).unwrap();
        let mut source = doc.block(from).unwrap().clone();
        source
            .card
            .as_mut()
            .unwrap()
            .publication
            .as_mut()
            .unwrap()
            .release = 3;
        source.entities.push(serde_json::from_value(json!({"kind":"insert","id":5,"layerId":"","attrs":{},"block":child.id,"p":{"x":0.3,"y":0.2},"scale":1.0,"rotation":0.0})).unwrap());
        if let Entity::Line(line) = &mut child.entities[0] {
            line.b.x = 0.3;
        }
        let library =
            json!({"format":"kentos.blocks","version":2,"blocks":[source,child]}).to_string();
        let input = BlocksReplace {
            library: Some(library),
            from: Some(from),
            to: Some(from),
            uids: Some(vec![uid]),
            purge: None,
            expected_revision: None,
        };
        (doc, input)
    }
    fn bytes(doc: &Document) -> Vec<u8> {
        kentos_kcad::encode_verified(&doc.to_snapshot_v2()).unwrap()
    }
    fn completed(
        answer: CommandResult<kentos_commands::BlocksReplaced>,
    ) -> kentos_commands::BlocksReplaced {
        match answer {
            CommandResult::Completed { output, .. } => output,
            other => panic!("{other:?}"),
        }
    }
    #[test]
    fn upgrade_isolates_nested_definitions_preserves_overrides_and_reuses_the_exact_publication_with_single_undo()
     {
        let (mut doc, mut input) = setup();
        let old = bytes(&doc);
        let old_blocks = doc.blocks().to_vec();
        let first_uid = input.uids.as_ref().unwrap()[0].clone();
        let planned = completed(super::super::plan(&ExecutionContext::new(&mut doc), &input));
        assert_eq!(bytes(&doc), old);
        assert_eq!(planned.added.len(), 2);
        let result = completed(super::super::execute(
            &mut ExecutionContext::new(&mut doc),
            input.clone(),
        ));
        assert_eq!(result.target, planned.target);
        assert_eq!(result.added, planned.added);
        assert_eq!(result.replaced, vec![first_uid.clone()]);
        for old in &old_blocks {
            assert_eq!(doc.block(old.id).unwrap(), &**old);
        }
        let first = bytes(&doc);
        let upgraded = doc
            .entities()
            .find_map(|e| match e {
                Entity::Insert(i) if Some(i.block) == result.target => Some(i),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            upgraded.parameters["Genişlik"],
            kentos_commands::ParamValue::Number(1.0)
        );
        let second_uid = doc
            .entities_with_uids()
            .find(|(_, e)| matches!(e,Entity::Insert(i) if Some(i.block)==input.from))
            .unwrap()
            .0
            .to_string();
        input.uids = Some(vec![second_uid]);
        let reused = completed(super::super::execute(
            &mut ExecutionContext::new(&mut doc),
            input,
        ));
        assert_eq!(reused.target, result.target);
        assert!(reused.added.is_empty());
        doc.undo().unwrap();
        assert_eq!(bytes(&doc), first);
        doc.undo().unwrap();
        assert_eq!(bytes(&doc), old);
        doc.redo().unwrap();
        assert_eq!(bytes(&doc), first);
    }
    #[test]
    fn upgrade_refuses_older_unrelated_invalid_and_incompatible_publications_without_partial_import()
     {
        let (mut doc, input) = setup();
        let old = bytes(&doc);
        for case in 0..8 {
            let mut bad = input.clone();
            let mut library: serde_json::Value =
                serde_json::from_str(bad.library.as_ref().unwrap()).unwrap();
            match case {
                0 => library["blocks"][0]["card"]["publication"]["release"] = json!(2),
                1 => {
                    library["blocks"][0]["card"]["publication"]["familyId"] =
                        json!("11111111-1111-4111-8111-111111111111")
                }
                2 => {
                    library["blocks"][0]["card"]["publication"]["typeId"] =
                        json!("11111111-1111-4111-8111-111111111111")
                }
                3 => library["version"] = json!(1),
                4 => {
                    library["blocks"][0]
                        .as_object_mut()
                        .unwrap()
                        .remove("dynamic");
                }
                5 => bad.purge = Some(true),
                6 => library["blocks"][0]["units"] = json!("unitless"),
                _ => {
                    library["blocks"][0]["entities"][4]["block"] =
                        json!("11111111-1111-4111-8111-111111111111")
                }
            }
            bad.library = Some(library.to_string());
            assert!(
                matches!(
                    super::super::validate(&ExecutionContext::new(&mut doc), &bad),
                    CommandResult::Failed { .. }
                ),
                "{case}"
            );
            assert!(
                matches!(
                    super::super::plan(&ExecutionContext::new(&mut doc), &bad),
                    CommandResult::Failed { .. }
                ),
                "{case}"
            );
            assert!(
                matches!(
                    super::super::execute(&mut ExecutionContext::new(&mut doc), bad),
                    CommandResult::Failed { .. }
                ),
                "{case}"
            );
            assert_eq!(bytes(&doc), old, "{case}");
        }
    }
    #[test]
    fn upgrade_reuses_an_unchanged_published_dependency_across_parent_releases() {
        let (mut doc, mut input) = setup();
        let mut file: serde_json::Value =
            serde_json::from_str(input.library.as_ref().unwrap()).unwrap();
        file["blocks"][1]["card"] = json!({"version":1,"publication":{
            "familyId":"11111111-1111-4111-8111-111111111111",
            "typeId":"22222222-2222-4222-8222-222222222222","typeName":"Donatı","release":1}});
        input.library = Some(file.to_string());
        let first = completed(super::super::execute(
            &mut ExecutionContext::new(&mut doc),
            input.clone(),
        ));
        assert_eq!(first.added.len(), 2);
        file["blocks"][0]["card"]["publication"]["release"] = json!(4);
        input.library = Some(file.to_string());
        input.uids = Some(vec![
            doc.entities_with_uids()
                .find(|(_, e)| matches!(e,Entity::Insert(i) if Some(i.block)==input.from))
                .unwrap()
                .0
                .to_string(),
        ]);
        let next = completed(super::super::execute(
            &mut ExecutionContext::new(&mut doc),
            input,
        ));
        assert_eq!(next.added.len(), 1);
        assert_eq!(doc.blocks().len(), 5);
    }
    #[test]
    fn upgrade_guarded_plan_rejects_a_changed_source_and_does_not_overwrite_a_locally_edited_target()
     {
        let (mut doc, input) = setup();
        let call = |op, input: &BlocksReplace, plan: Option<String>| CallEnvelope {
            command: kentos_commands::CAD_BLOCKS_REPLACE.into(),
            version: None,
            op,
            input: serde_json::to_value(input).unwrap(),
            request_id: None,
            expected: plan.map(|plan| CallExpectations {
                plan: Some(plan),
                ..Default::default()
            }),
            relax_constraints: false,
        };
        let planned = crate::call::call(&mut doc, None, call(CallOp::Plan, &input, None));
        assert!(
            matches!(planned.result, CommandResult::Completed { .. }),
            "{planned:?}"
        );
        let old = bytes(&doc);
        let mut changed = input.clone();
        let mut file: serde_json::Value =
            serde_json::from_str(changed.library.as_ref().unwrap()).unwrap();
        file["blocks"][0]["description"] = json!("Changed since review");
        changed.library = Some(file.to_string());
        let rejected = crate::call::call(
            &mut doc,
            None,
            call(CallOp::Execute, &changed, planned.plan_id),
        );
        assert!(!matches!(rejected.result, CommandResult::Completed { .. }));
        assert_eq!(bytes(&doc), old);
        let output = completed(super::super::execute(
            &mut ExecutionContext::new(&mut doc),
            input.clone(),
        ));
        let mut local = doc.block(output.target.unwrap()).unwrap().clone();
        local.description = Some("Yerel düzenleme".into());
        doc.update_block(local).unwrap();
        let before = bytes(&doc);
        let mut next = input;
        next.uids = Some(vec![
            doc.entities_with_uids()
                .find(|(_, e)| matches!(e,Entity::Insert(i) if Some(i.block)==next.from))
                .unwrap()
                .0
                .to_string(),
        ]);
        assert!(matches!(
            super::super::execute(&mut ExecutionContext::new(&mut doc), next),
            CommandResult::Failed { .. }
        ));
        assert_eq!(bytes(&doc), before);
    }
}
