//! Verified, selected repeat conversion. Geometry recognition is read-only; this command
//! separately guards appearance/data, builds root-to-piece maps and remaps ordinary dimensions.
use crate::{
    ExecutionContext,
    checks::{self, Stop, error},
};
use kentos_commands::{
    BlockDefinition, BlockId, BlocksConvert, BlocksConvertPlan, BlocksConverted, BlocksFind,
    CommandResult, Entity, EntityBase, EntityId, InsertEntity, RECOGNITION_SCOPE_MAX,
    RECOGNITION_TOLERANCE, RepeatDimensionMap, RepeatMemberMap, RepeatPlacement, SubRef,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::ops::{
    repeats::{self, PlaceMatch},
    subref,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

mod relations;
mod values;
pub const LABEL: &str = "Tekrarları bloğa dönüştür";

fn refuse(message: impl Into<String>, path: impl Into<String>) -> Stop {
    Stop::failed(error(
        "recognition_conversion_refused",
        message.into(),
        Some(path.into()),
    ))
}
fn key(group: &[String]) -> Vec<String> {
    let mut sorted = group.to_vec();
    sorted.sort();
    sorted
}
fn uuid(text: &str) -> Result<Uuid, Stop> {
    Uuid::parse_str(text).map_err(|_| {
        refuse(
            "Adayın nesne kimliği okunamadı; aramayı yenileyin.",
            "groups",
        )
    })
}
fn entity<'a>(doc: &'a Document, text: &str) -> Result<(&'a Entity, Slot), Stop> {
    let slot = doc
        .slot_of(uuid(text)?)
        .ok_or_else(|| refuse("Aday nesne çizimde yok; aramayı yenileyin.", "groups"))?;
    let e = doc
        .get(slot)
        .ok_or_else(|| refuse("Aday nesne çizimde yok; aramayı yenileyin.", "groups"))?;
    Ok((e, slot))
}

struct Checked {
    plan: BlocksConvertPlan,
    dimensions: Vec<(Slot, Entity)>,
}

fn check(
    doc: &Document,
    input: &BlocksConvert,
    stopped: &mut impl FnMut() -> bool,
) -> Result<Checked, Stop> {
    if stopped() {
        return Err(Stop::Cancelled);
    }
    crate::blocks_define::name(&input.name)?;
    if input.source.is_empty()
        || input.source.len() > 128
        || input.groups.is_empty()
        || input.groups.len() > 1000
    {
        return Err(refuse(
            "Bir kaynak bileşen ve dönüştürülecek en çok 1 000 aday seçin; kaynak 1–128 nesne olabilir.",
            "groups",
        ));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    // A dependency review must see the whole document, not only the selected candidate scope.
    if doc.len() > RECOGNITION_SCOPE_MAX {
        return Err(refuse(
            "Bu dönüşümün ilişki denetimi en çok 50 000 nesneli çizimleri destekler. Çizim değiştirilmedi.",
            "groups",
        ));
    }
    let mut selected = HashSet::new();
    for (i, g) in input.groups.iter().enumerate() {
        if g.len() != input.source.len() {
            return Err(refuse(
                "Her aday kaynakla aynı sayıda nesne içermeli.",
                format!("groups[{i}]"),
            ));
        }
        for uid in g {
            if !selected.insert(uid.clone()) {
                return Err(refuse(
                    "Seçilen adaylar nesne paylaşıyor veya aynı nesne iki kez verilmiş. Çakışan adaylardan birini seçimden çıkarın.",
                    format!("groups[{i}]"),
                ));
            }
        }
    }
    if selected.len() > RECOGNITION_SCOPE_MAX {
        return Err(refuse(
            "Seçilen adaylar en çok 50 000 nesne olabilir.",
            "groups",
        ));
    }
    let source_set: HashSet<_> = input.source.iter().collect();
    if source_set.len() != input.source.len() {
        return Err(refuse("Kaynakta aynı nesne iki kez verilmiş.", "source"));
    }
    // Verify all requested identities through the common command before using them below.
    let mut scope: Vec<_> = selected.iter().cloned().collect();
    scope.sort();
    let mut query = BlocksFind {
        source: input.source.clone(),
        uids: Some(scope),
        rotations: input.rotations,
        mirrors: input.mirrors,
        scales: input.scales,
        tolerance: input.tolerance,
        geometry_only: Some(false),
        text_variants: input.text_to_attributes,
        attribute_variants: Some(false),
        limit: Some(1000),
        expected_revision: input.expected_revision.clone(),
    };
    // Membership input order must not change how a definition paints its pieces.
    let sorted_source: Vec<_> = doc
        .entities_with_uids()
        .map(|(u, _)| u.to_string())
        .filter(|u| source_set.contains(u))
        .collect();
    if sorted_source.len() == input.source.len() {
        query.source = sorted_source;
    }
    let found = match crate::blocks_find::inspect(doc, &query, &mut *stopped) {
        CommandResult::Completed { output, .. } => output,
        CommandResult::Failed { error } => return Err(Stop::failed(error)),
        CommandResult::Conflict { error } => return Err(Stop::conflict(error)),
        CommandResult::Cancelled => return Err(Stop::Cancelled),
        _ => {
            return Err(refuse(
                "Adaylar doğrulanamadı; çizim değiştirilmedi.",
                "groups",
            ));
        }
    };
    let candidates: HashMap<_, _> = found.matches.iter().map(|m| (key(&m.members), m)).collect();
    let mut matched = Vec::new();
    for (i, g) in input.groups.iter().enumerate() {
        let m = candidates.get(&key(g)).ok_or_else(|| {
            let mut message = "Seçilen aday çizimin şimdiki durumunda tam doğrulanamadı. Geometri, katman, görünüş veya öznitelik farklı olabilir; aramayı yenileyin.".to_owned();
            if input.text_to_attributes != Some(true) && found.source.iter().any(|u| entity(doc,u).is_ok_and(|(e,_)|matches!(e,Entity::Text(_)))) {
                message.push_str(" Yazı değerleri farklıysa Metin değerlerini blok öznitelikleri olarak koru seçeneğini açıp yeniden inceleyin; biçim ve diğer farklar yine denetlenir.");
            }
            refuse(message,format!("groups[{i}]"))
        })?;
        matched.push(*m);
    }
    let source: Vec<_> = found
        .source
        .iter()
        .map(|u| entity(doc, u).map(|(e, _)| e.clone()))
        .collect::<Result<Vec<_>, _>>()?;
    let layer = source[0].base().layer_id.clone();
    let all: HashSet<_> = selected
        .iter()
        .cloned()
        .chain(found.source.iter().cloned())
        .collect();
    relations::appearance(doc, &all, &layer, stopped)?;
    let (pieces,attributes)=values::definitions(&source,input.text_to_attributes.unwrap_or(false))?;
    let entities = crate::blocks_define::as_drawn(crate::blocks_define::copies(pieces.iter()));
    let block = BlockDefinition {
        id: BlockId([0; 16]),
        name: input.name.clone(),
        base: matched[0].origin,
        units: Some(kentos_commands::BlockUnits::M),
        entities,
        attributes,
        description: None,
        dynamic: None,
        xref: None,
        card: None,
    };
    let mut defs = doc.blocks().to_vec();
    let definition = Arc::new(block.clone());
    defs.push(Arc::clone(&definition));
    let refs: Vec<_> = defs.iter().map(|b| &**b).collect();
    crate::blocks_define::block_rules(&refs)?;
    let mut needed=vec![Arc::clone(&definition)];let mut pending:Vec<_>=source.iter().filter_map(|e|if let Entity::Insert(i)=e {Some(i.block)} else {None}).collect();
    let mut seen=HashSet::new();let mut total=source.len();
    while let Some(id)=pending.pop() {
        if stopped() {return Err(Stop::Cancelled);}
        if !seen.insert(id) {continue;}
        let Some(inner)=doc.block(id) else{return Err(refuse("İç blok tanımı yok.","source"));};
        total=total.saturating_add(inner.entities.len());
        if total>50_000 || inner.dynamic.is_some() || inner.xref.is_some() {return Err(refuse("İç blok bu kapsamda statik olmalı ve toplam tanım içeriği 50000 nesneyi aşmamalı. Tanım değiştirilmedi.","source"));}
        pending.extend(inner.entities.iter().filter_map(|e|if let Entity::Insert(i)=e {Some(i.block)} else {None}));
        needed.push(Arc::new(inner.clone()));
    }
    let core_blocks = crate::blocks::core_blocks_in(&needed, crate::geometry::drawing_font(doc.settings().drawing_font));
    let paint_order = relations::PaintOrder::new(doc, &layer);
    let mut mapping = Vec::new();
    let mut placements = Vec::new();
    let mut removed = Vec::new();
    for (g, m) in matched.iter().enumerate() {
        if stopped() {
            return Err(Stop::Cancelled);
        }
        // Regrouping interleaved objects or changing piece paint order can change masks/overlap.
        paint_order.check(&m.members, g)?;
        let first = &m.members[0];
        let (old, slot) = entity(doc, first)?;
        let entity = Entity::Insert(InsertEntity {
            base: EntityBase {
                id: slot.0,
                layer_id: layer.clone(),
                color: None,
                attrs: values::attributes(doc,&m.members,&source,&block.attributes)?,
                label: None,
                symbol: None,
                line_weight: None,
                look: kentos_commands::ObjectLook {
                    order: old.base().look.order,
                    ..Default::default()
                },
                label_pins: Vec::new(),
                attachments: Vec::new(),
            },
            block: block.id,
            p: m.at,
            scale: m.scale,
            y_scale: None,
            rotation: m.rotation,
            mirror: m.mirror,
            parameters: Default::default(),
            attribute_places: Default::default(),
            clipping: None,
            arch: Default::default(),
        });
        placements.push(RepeatPlacement {
            uid: first.clone(),
            entity,
        });
        for (i, from) in m.members.iter().enumerate() {
            let attribute=if input.text_to_attributes==Some(true) && matches!(source[i],Entity::Text(_)) {
                block.attributes.get(i-block.entities.len()).map(|a|a.tag.clone())
            } else {None};
            mapping.push(RepeatMemberMap {
                from: from.clone(),
                insert: first.clone(),
                path: if attribute.is_some() {Vec::new()} else {vec![i as u32]},
                attribute,
            });
            if from != first {
                removed.push(from.clone());
            }
        }
    }
    let by_uid: HashMap<_, _> = mapping.iter().map(|m| (m.from.clone(), m)).collect();
    let by_insert: HashMap<_, _> = placements.iter().map(|p| (p.uid.clone(), p)).collect();
    let mut dimensions = Vec::new();
    let mut changes = Vec::new();
    let tolerance = input.tolerance.unwrap_or(RECOGNITION_TOLERANCE);
    let mut work = 0u64;
    // Check the actual attribute values at their final world poses, including readable reflection.
    if !block.attributes.is_empty() {
        for placed in &placements {
            if stopped() {return Err(Stop::Cancelled);}
            let expanded=core_blocks.expand(&crate::geometry::shape(&placed.entity));
            work+=expanded.len() as u64;
            if work>2_000_000 {return Err(refuse("Öznitelik görünüş denetimi iş sınırını aştı; çizim değiştirilmedi.","groups"));}
            for map in mapping.iter().filter(|m|m.insert==placed.uid && m.attribute.is_some()) {
                let Some(tag)=&map.attribute else{continue;};
                let (target,_)=entity(doc,&map.from)?;
                let mut matching=expanded.iter().filter(|p|p.attribute.as_ref()==Some(tag));
                let verified=matching.next().is_some_and(|p|repeats::equivalent(&p.shape,&crate::geometry::shape(target),input.tolerance.unwrap_or(RECOGNITION_TOLERANCE),&mut ||{work+=1;work<=2_000_000 && !stopped()})==Some(true));
                if stopped() {return Err(Stop::Cancelled);}
                if !verified || matching.next().is_some() {return Err(refuse("Özniteliğin son konumu, yönü veya yazı biçimi özgün yazıyla aynı doğrulanamadı; değer kaybedilmedi.","groups"));}
            }
        }
    }
    for (dim_uid, e) in doc.entities_with_uids() {
        if stopped() {
            return Err(Stop::Cancelled);
        }
        let Entity::Dimension(d) = e else {
            continue;
        };
        let Some(held) = &d.refs else {
            continue;
        };
        if ![&held.a, &held.b, &held.c].iter().any(|r| {
            r.as_ref()
                .is_some_and(|r| by_uid.contains_key(&r.uid.to_text()))
        }) {
            continue;
        }
        if held.rule.is_some() {
            return Err(refuse(
                "Bu adaya türetilmiş ölçü kuralı bağlı. Kuralın kaynak grafiği henüz bu dönüşümde taşınmıyor; ölçü bağı silinmedi.",
                format!("dimensions.{}", dim_uid),
            ));
        }
        if doc.layers().is_locked(&d.base.layer_id) {
            return Err(Stop::failed(error(
                "layer_locked",
                "Adaya bağlı ölçünün katmanı kilitli; ölçü bağı güncellenemedi.".into(),
                Some(format!("dimensions.{dim_uid}")),
            )));
        }
        let mut next = d.clone();
        let Some(refs) = next.refs.as_mut() else {
            continue;
        };
        for (name, held) in [("a", &mut refs.a), ("b", &mut refs.b), ("c", &mut refs.c)] {
            let Some(before) = held.as_ref() else {
                continue;
            };
            let Some(map) = by_uid.get(&before.uid.to_text()) else {
                continue;
            };
            if map.attribute.is_some() {return Err(refuse("Bu yazıya ölçü alt nesne bağı var; öznitelik hedefi aynı SubRef yolu değildir. Ölçü bağı silinmedi.",format!("dimensions.{dim_uid}.{name}")));}
            if before.broken.is_some() {
                return Err(refuse(
                    "Adayın ölçü bağı zaten kopuk veya beklenmeyen bir alt nesne yolu taşıyor. Önce bağı kontrol edin.",
                    format!("dimensions.{dim_uid}.{name}"),
                ));
            }
            let (target, _) = entity(doc, &map.from)?;
            if !before.path.is_empty() && !matches!(target,Entity::Insert(_)) {
                return Err(refuse("Alt nesne yolu yalnız mevcut iç blokta izlenebilir; ölçü bağı değiştirilmedi.",format!("dimensions.{dim_uid}.{name}")));
            }
            let mut path=map.path.clone();path.extend_from_slice(&before.path);
            let placed = &by_insert
                .get(&map.insert)
                .ok_or_else(|| refuse("Yeni yerleştirme eşlemesi eksik.", "groups"))?
                .entity;
            let piece = subref::piece(&core_blocks, &crate::blocks::core_entity(placed), &path)
                .ok_or_else(|| {
                    refuse(
                        "Yeni blokta ölçünün karşılık geldiği parça bulunamadı.",
                        "groups",
                    )
                })?;
            let root = crate::blocks::core_entity(target);
            let target = if before.path.is_empty() {root} else {
                subref::piece(&core_blocks,&root,&before.path).ok_or_else(||refuse("Özgün iç blokta ölçü yolu bulunamadı.",format!("dimensions.{dim_uid}.{name}")))?
            };
            let mapped = repeats::map_place(
                &piece.shape,
                &target.shape,
                kentos_domain::dimension_ties::core_place(before.place),
                tolerance,
                &mut || {
                    work += 1;
                    work <= 2_000_000 && !stopped()
                },
            );
            let place = match mapped {
                PlaceMatch::At(p) => p,
                PlaceMatch::Stopped if stopped() => return Err(Stop::Cancelled),
                PlaceMatch::Stopped => {
                    return Err(refuse(
                        "Ölçü eşleme iş sınırına ulaştı; hiçbir bağ değiştirilmedi. Daha az aday seçin.",
                        "groups",
                    ));
                }
                PlaceMatch::Ambiguous => {
                    return Err(refuse(
                        "Aynı geometriye giden birden fazla köşe/kenar eşlemesi var. Ölçü için rastgele bir kenar seçilmedi.",
                        format!("dimensions.{dim_uid}.{name}"),
                    ));
                }
                PlaceMatch::NoMatch => {
                    return Err(refuse(
                        "Ölçünün bağlı olduğu yer yeni blok parçasında doğrulanamadı.",
                        format!("dimensions.{dim_uid}.{name}"),
                    ));
                }
            };
            let old = subref::point(
                &target,
                kentos_domain::dimension_ties::core_place(before.place),
            )
            .ok_or_else(|| refuse("Ölçünün eski kaynak noktası bulunamadı.", "dimensions"))?;
            let at = subref::point(&piece, place).ok_or_else(|| {
                refuse("Ölçünün yeni alt nesne noktası bulunamadı.", "dimensions")
            })?;
            let stored = match name {
                "a" => Some(d.a),
                "b" => Some(d.b),
                _ => d.c,
            }
            .ok_or_else(|| refuse("Ölçünün kaynak nokta kaydı eksik.", "dimensions"))?;
            let previous = kentos_geometry_core::Vec2::new(stored.x, stored.y);
            let deviation = kentos_geometry_core::geometry::dist(previous, at);
            if !deviation.is_finite()
                || deviation > tolerance
                || kentos_geometry_core::geometry::dist(previous, old) > tolerance
            {
                return Err(refuse(
                    "Bağlı ölçü noktası eşleme toleransından fazla yer değiştirecek veya kaynak bağı güncel değil. Ölçüyü kontrol edin; nokta sessizce taşınmadı.",
                    format!("dimensions.{dim_uid}.{name}"),
                ));
            }
            let after = SubRef {
                uid: EntityId(uuid(&map.insert)?.into_bytes()),
                path,
                place: kentos_domain::dimension_ties::contract_place(place),
                broken: None,
            };
            dimensions.push(RepeatDimensionMap {
                uid: dim_uid.to_string(),
                point: name.into(),
                before: before.clone(),
                after: after.clone(),
                deviation,
            });
            *held = Some(after);
            let at = kentos_commands::Vec2 { x: at.x, y: at.y };
            match name {
                "a" => next.a = at,
                "b" => next.b = at,
                _ => next.c = Some(at),
            };
        }
        changes.push((Slot(d.base.id), Entity::Dimension(next)));
    }
    relations::dependencies(doc, &selected, &layer, &dimensions, stopped)?;
    Ok(Checked {
        plan: BlocksConvertPlan {
            block,
            placements,
            mapping,
            dimensions,
            rebound: changes.iter().map(|(_, e)| e.clone()).collect(),
            removed,
            revision: doc.revision().to_string(),
        },
        dimensions: changes,
    })
}

/// Cancellable read-only review, also used by the desktop worker.
pub fn inspect(
    doc: &Document,
    input: &BlocksConvert,
    mut stopped: impl FnMut() -> bool,
) -> CommandResult<BlocksConvertPlan> {
    match check(doc, input, &mut stopped) {
        Ok(c) => CommandResult::Completed {
            output: c.plan,
            warnings: Vec::new(),
        },
        Err(s) => s.into(),
    }
}
pub fn validate(cx: &ExecutionContext<'_>, input: &BlocksConvert) -> CommandResult<()> {
    match check(cx.doc, input, &mut || cx.stopped()) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(s) => s.into(),
    }
}
pub fn plan(cx: &ExecutionContext<'_>, input: &BlocksConvert) -> CommandResult<BlocksConvertPlan> {
    inspect(cx.doc, input, || cx.stopped())
}
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: BlocksConvert,
) -> CommandResult<BlocksConverted> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_BLOCKS_CONVERT,
        kentos_commands::CAD_BLOCKS_CONVERT_VERSION,
        input,
        write,
    )
}
fn write(cx: &mut ExecutionContext<'_>, input: BlocksConvert) -> CommandResult<BlocksConverted> {
    let mut checked = match check(cx.doc, &input, &mut || cx.stopped()) {
        Ok(c) => c,
        Err(s) => return s.into(),
    };
    if cx.stopped() {
        return CommandResult::Cancelled;
    }
    let id = BlockId(Uuid::now_v7().into_bytes());
    checked.plan.block.id = id;
    let mut changes = checked.dimensions;
    for p in &mut checked.plan.placements {
        if let Entity::Insert(i) = &mut p.entity {
            i.block = id;
        }
        changes.push((Slot(p.entity.base().id), p.entity.clone()));
    }
    let removed = match checked
        .plan
        .removed
        .iter()
        .map(|u| entity(cx.doc, u).map(|(_, s)| s))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(v) => v,
        Err(s) => return s.into(),
    };
    let written = cx.doc.transact(LABEL, |doc| {
        doc.add_block(checked.plan.block)
            .map_err(|e| e.to_string())?;
        doc.update_many(changes, LABEL);
        doc.remove(&removed);
        Ok::<_, String>(())
    });
    if let Err(message) = written {
        return Stop::failed(error("block_refused", message, None)).into();
    }
    CommandResult::Completed {
        output: BlocksConverted {
            block: id,
            inserts: checked
                .plan
                .placements
                .iter()
                .map(|p| p.uid.clone())
                .collect(),
            mapping: checked.plan.mapping,
            dimensions: checked.plan.dimensions,
            removed: checked.plan.removed,
            revision: cx.doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}
