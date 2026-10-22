//! Bound layers written to one PostGIS database in one transaction (docs/adr/0582 §2, 0584 §1). Every layer is read
//! and checked first (read-only table, the area changed under a layer without a base, geometry the column cannot hold,
//! Z and M the column does not take); then the tables are locked in their names' order, each one's touched rows are
//! checked again against the layer's base, and all of them are written in one transaction or none. Layers of another
//! database or of a file are not written with them: no transaction spans two sources.

use std::collections::HashMap;

use kentos_commands::{
    CommandWarning, SourceKind, SourceLink, SourcesSynced, SyncTopology, SyncedLayer,
    SyncedRelation,
};
use kentos_domain::{Document, Uuid};
use kentos_geopackage::{self as gpkg, Edits};

use super::relations::{self, Kept, Side};
use super::together::{Loaded, load_link, skipped};
use super::{
    Diff, Followed, Mover, Stored, at_one, base, codes, conflict, conflict_on, follow_outside,
    layer_digest, mine_only, neighbours_changed, provider_stop, refuse, refuse_on, rows_changed,
    set_link,
};
use crate::checks::Stop;
use crate::provider;

/// Where a refusal points: `layer` for the first layer, `with` for the others.
fn at(i: usize) -> &'static str {
    if i == 0 { "layer" } else { "with" }
}

/// A layer of the write, read and checked.
struct Item {
    link: SourceLink,
    stored: Stored,
    mover: Option<Mover>,
    area: Option<[f64; 4]>,
    base: Option<HashMap<Uuid, base::BaseObj>>,
    diff: Diff,
    followed: Followed,
    /// Someone else's rows in the area since the layer last matched the table.
    others: bool,
}

/// Writes the PostGIS layers `layers` (the first is the one asked, the rest `with`) to their database in one
/// transaction; with `write` false, only says what it would do.
pub(super) fn write(
    doc: &mut Document,
    layers: &[String],
    write: bool,
    topology: Option<SyncTopology>,
) -> Result<(SourcesSynced, Vec<CommandWarning>), Stop> {
    let mut links: Vec<SourceLink> = Vec::with_capacity(layers.len());
    for (i, id) in layers.iter().enumerate() {
        if layers[..i].contains(id) {
            return Err(refuse(
                codes::INVALID_INPUT,
                format!("“{id}” katmanı birden çok kez verildi; her katman bir kez yazılır."),
                at(i),
            ));
        }
        let Some(link) = doc
            .settings()
            .sources
            .iter()
            .find(|s| s.layer == *id)
            .cloned()
        else {
            return Err(refuse(
                codes::SOURCE_NOT_FOUND,
                format!(
                    "“{id}” katmanı bir kaynağa bağlı değil; önce cad.sources.edit ile bağlayın."
                ),
                at(i),
            ));
        };
        if doc.layers().get(&link.layer).is_none() {
            return Err(refuse(
                codes::LAYER_NOT_FOUND,
                format!(
                    "Bağlı “{}” katmanı çizimde yok; bağı çözün (cad.sources.edit unlink).",
                    link.layer
                ),
                at(i),
            ));
        }
        links.push(link);
    }
    let address = links[0].path.clone();
    for (i, l) in links.iter().enumerate() {
        if l.kind != SourceKind::Postgis || l.path != address {
            return Err(refuse(
                codes::TRANSACTION_SPLIT,
                format!(
                    "“{}” katmanı {} kaynağında, “{}” katmanı {address} veritabanında; birlikte yazılacak katmanlar tek veritabanında olmalı, iki kaynağa birden tek işlem olmaz. Katmanları ayrı ayrı yazın.",
                    l.layer, l.path, links[0].layer
                ),
                at(i),
            ));
        }
    }
    // Many-to-many references of the database's junction tables (docs/adr/0590 §3): not columns, rows of their table.
    let ties = relations::ties(doc);
    let mut items: Vec<Item> = Vec::with_capacity(links.len());
    for (i, link) in links.into_iter().enumerate() {
        let Loaded {
            stored,
            mover,
            area,
        } = load_link(doc, &link, at(i))?;
        if !link.writable || !stored.writable {
            return Err(refuse(
                codes::SOURCE_READONLY,
                format!(
                    "{} veritabanının “{}” tablosuna yazılamaz (salt okunur).",
                    link.path, link.table
                ),
                at(i),
            ));
        }
        // Row by row against the layer's base (docs/adr/0582 §2); without one, the area as a whole against its revision.
        let base = base::of(doc, &link);
        if base.is_none() && stored.sha256 != link.revision {
            return Err(conflict(
                codes::SOURCE_CHANGED,
                format!(
                    "{} katman onunla son eşleştiğinden beri değişti; üzerine yazılmadı. İki yanı cad.sources.reconcile ile uzlaştırın ya da tablodan okuyun (direction read).",
                    link.table
                ),
                at(i),
            ));
        }
        let mut d = super::diff_skipping(
            doc,
            &link.layer,
            &stored,
            mover.as_ref(),
            &skipped(&ties, &link.layer),
        );
        let mut others = stored.sha256 != link.revision;
        if let Some(b) = &base {
            others |= mine_only(doc, &link.layer, &stored, b, &mut d)?;
        }
        if let Some((uid, why)) = &d.mismatch {
            return Err(refuse(
                codes::GEOMETRY_MISMATCH,
                format!("{uid} kimlikli nesne tabloya yazılamaz: {why}."),
                at(i),
            ));
        }
        let followed = match topology {
            None => Followed::default(),
            Some(profile) => {
                let mut columns = stored.layer.columns.clone();
                columns.extend(d.added_columns.iter().cloned());
                follow_outside(
                    doc,
                    &link,
                    super::origin_from(link.kind, &link.path),
                    area,
                    &stored,
                    &columns,
                    mover.as_ref(),
                    profile,
                )?
            }
        };
        items.push(Item {
            link,
            stored,
            mover,
            area,
            base,
            diff: d,
            followed,
            others,
        });
    }
    let mut warnings = said(&items);
    let (maps, synced_relations) = junctions(doc, &ties, layers, &items)?;
    for note in relations::unmapped_notes(&ties, layers) {
        warnings.push(CommandWarning {
            code: codes::RELATION_UNMAPPED.into(),
            message: format!("{note}."),
            path: Some("layer".into()),
        });
    }
    let layers_out: Vec<SyncedLayer> = items
        .iter()
        .map(|it| SyncedLayer {
            layer: it.link.layer.clone(),
            added: it.diff.insert.len() as u32,
            modified: it.diff.update.len() as u32,
            removed: it.diff.delete.len() as u32,
        })
        .collect();
    let mut out = SourcesSynced {
        added: layers_out.iter().map(|l| l.added).sum(),
        modified: layers_out.iter().map(|l| l.modified).sum(),
        removed: layers_out.iter().map(|l| l.removed).sum(),
        columns: items
            .iter()
            .flat_map(|it| it.diff.added_columns.iter().map(|c| c.name.clone()))
            .collect(),
        source: items[0].stored.sha256.clone(),
        pins: None,
        revision: doc.revision().to_string(),
        layers: if items.len() > 1 {
            layers_out
        } else {
            Vec::new()
        },
        relations: synced_relations,
        followed: items.iter().map(|it| it.followed.rows.len() as u32).sum(),
        left: items.iter().map(|it| it.followed.left).sum(),
    };
    if !write {
        return Ok((out, warnings));
    }
    // The tables that change, their edits.
    let mut jobs: Vec<(usize, Edits)> = Vec::new();
    for (i, it) in items.iter_mut().enumerate() {
        let d = &mut it.diff;
        if d.is_empty() && d.added_columns.is_empty() && it.followed.rows.is_empty() {
            continue;
        }
        let mut update = std::mem::take(&mut d.update);
        update.extend(std::mem::take(&mut it.followed.rows));
        update.sort_by_key(|(fid, _)| *fid);
        jobs.push((
            i,
            Edits {
                add_columns: std::mem::take(&mut d.added_columns),
                update,
                insert: std::mem::take(&mut d.insert),
                delete: std::mem::take(&mut d.delete),
            },
        ));
    }
    if !jobs.is_empty() || !maps.is_empty() {
        let p = super::database_provider("layer")?;
        // A row a table of this write inserts is named by the table's place among the jobs.
        let place: HashMap<usize, usize> =
            jobs.iter().enumerate().map(|(k, (i, _))| (*i, k)).collect();
        let key = |k: gpkg::relations::Key| match k {
            gpkg::relations::Key::Inserted(i, n) => {
                gpkg::relations::Key::Inserted(place.get(&i).copied().unwrap_or(usize::MAX), n)
            }
            k => k,
        };
        let maps: Vec<provider::MapWrite> = maps
            .into_iter()
            .map(|m| provider::MapWrite {
                mapping: m.mapping,
                bases: m.bases.into_iter().map(key).collect(),
                pairs: m.pairs.into_iter().map(|(b, r)| (key(b), key(r))).collect(),
            })
            .collect();
        let fid_uids: Vec<HashMap<i64, Uuid>> = jobs
            .iter()
            .map(|(i, _)| {
                items[*i]
                    .stored
                    .rows
                    .iter()
                    .map(|(u, (f, ..))| (*f, *u))
                    .collect()
            })
            .collect();
        // What each table's check found, for the refusal: the layer's own rows, and the neighbours outside its area.
        let mut changed: Vec<(Vec<Uuid>, Vec<Uuid>)> = vec![(Vec::new(), Vec::new()); jobs.len()];
        let edited = {
            let mut checks: Vec<Box<provider::RowCheck<'_>>> = Vec::with_capacity(jobs.len());
            for ((k, (i, edits)), slot) in jobs.iter().enumerate().zip(changed.iter_mut()) {
                let it = &items[*i];
                let fid_uid = &fid_uids[k];
                let base = it.base.as_ref();
                let mover = it.mover.as_ref();
                let expect = &it.followed.expect;
                checks.push(Box::new(move |layer: &gpkg::Layer, rows: &[gpkg::Row]| {
                    let mut keys: Vec<i64> = edits.update.iter().map(|(f, _)| *f).collect();
                    keys.extend(edits.delete.iter().copied());
                    let mut bad = rows_changed(base, layer, rows, &keys, fid_uid, mover);
                    slot.0 = bad.iter().filter_map(|f| fid_uid.get(f).copied()).collect();
                    let near = neighbours_changed(expect, layer, rows, mover);
                    slot.1 = near
                        .iter()
                        .filter_map(|f| expect.get(f).map(|(u, _)| *u))
                        .collect();
                    bad.extend(near);
                    if bad.is_empty() { Ok(()) } else { Err(bad) }
                }));
            }
            let tables: Vec<provider::TableEdit<'_, '_>> = jobs
                .iter()
                .zip(checks.iter_mut())
                .map(|((i, edits), check)| {
                    let it = &items[*i];
                    // Row by row with a base; without one the area as a whole too, and the neighbours row by row.
                    let rows = it.base.is_some() || !it.followed.expect.is_empty();
                    provider::TableEdit {
                        table: &it.link.table,
                        area: it.area,
                        revision: if it.base.is_some() {
                            ""
                        } else {
                            &it.link.revision
                        },
                        check: rows.then_some(&mut **check),
                        edits,
                    }
                })
                .collect();
            p.edit_many_mapped(&address, tables, &maps)
        };
        let edited = edited.map_err(|(k, e)| {
            if k >= jobs.len() {
                return provider_stop(e, &address, at_one("layer"));
            }
            let it = &items[jobs[k].0];
            let place = at(jobs[k].0);
            let (own, near) = &changed[k];
            match e {
                provider::ProviderError::RowsChanged(_) if own.is_empty() && !near.is_empty() => {
                    conflict_on(
                        codes::SOURCE_CHANGED,
                        format!(
                            "“{}” tablosunda alanın dışındaki {} komşu satırı, okunduktan sonra başka biri değiştirdi; hiçbir tablo yazılmadı. Yeniden yazın: komşular yeni hâlleriyle okunur.",
                            it.link.table,
                            near.len()
                        ),
                        "topology",
                        near,
                    )
                }
                provider::ProviderError::RowsChanged(_) => {
                    let all: Vec<Uuid> = own.iter().chain(near).copied().collect();
                    conflict_on(
                        codes::SOURCE_CHANGED,
                        format!(
                            "“{}” tablosunun {} satırını, katman onlarla son eşleştiğinden beri başka biri değiştirdi; hiçbir tablo yazılmadı. Tablodan okuyun (direction read) ya da o nesnelerdeki değişikliği bırakın.",
                            it.link.table,
                            all.len()
                        ),
                        place,
                        &all,
                    )
                }
                provider::ProviderError::RowsDenied(keys) => {
                    let fid_uid = &fid_uids[k];
                    let uids: Vec<Uuid> = keys
                        .iter()
                        .filter_map(|f| {
                            fid_uid
                                .get(f)
                                .or_else(|| it.followed.expect.get(f).map(|(u, _)| u))
                                .copied()
                        })
                        .collect();
                    let neighbours = keys.iter().all(|f| it.followed.expect.contains_key(f));
                    refuse_on(
                        codes::ROWS_DENIED,
                        format!(
                            "“{}” tablosunun {}{} satırını bu hesap görür ama değiştiremez (yetki ya da satır güvenliği); hiçbir tablo yazılmadı. Veritabanının yöneticisinden yetki isteyin{}.",
                            it.link.table,
                            if neighbours { "alanın dışındaki " } else { "" },
                            keys.len(),
                            if neighbours {
                                " ya da komşuları izlemeden yazın (topology vermeden)"
                            } else {
                                ""
                            }
                        ),
                        if neighbours { "topology" } else { place },
                        &uids,
                    )
                }
                e => provider_stop(e, &address, at_one(place)),
            }
        })?;
        for ((i, _), e) in jobs.iter().zip(edited) {
            let it = &mut items[*i];
            it.others |= e.before != it.link.revision;
            // Someone else's rows in the area keep the link “changed” until a read takes them.
            if !it.others || it.base.is_none() {
                it.link.revision = e.after;
            }
        }
    }
    for it in &mut items {
        it.link.digest = layer_digest(doc, &it.link.layer);
        set_link(doc, Some(it.link.clone()), None);
        base::after_write(doc, &it.link);
    }
    out.source = items[0].link.revision.clone();
    out.revision = doc.revision().to_string();
    Ok((out, warnings))
}

/// The junction tables' rows the write rewrites, and what changes in each: for every KentOS junction a layer of the
/// write is a side of, the rows of the children the layers hold, made from their references (docs/adr/0590 §3).
fn junctions(
    doc: &Document,
    ties: &[relations::Tie],
    layers: &[String],
    items: &[Item],
) -> Result<(Vec<provider::MapWrite>, Vec<SyncedRelation>), Stop> {
    let touching = relations::mapped_touching(ties, layers);
    if touching.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let p = super::database_provider("layer")?;
    let inserts: Vec<HashMap<Uuid, usize>> = items
        .iter()
        .map(|it| {
            it.diff
                .insert
                .iter()
                .enumerate()
                .filter_map(|(i, (u, _))| Uuid::parse_str(u).ok().map(|u| (u, i)))
                .collect()
        })
        .collect();
    let mut partners: HashMap<String, Stored> = HashMap::new();
    for t in &touching {
        for (layer, link) in [(&t.child_layer, &t.child), (&t.parent_layer, &t.parent)] {
            if !layers.contains(layer) && !partners.contains_key(layer) {
                partners.insert(layer.clone(), load_link(doc, link, "layer")?.stored);
            }
        }
    }
    let mut maps = Vec::new();
    let mut synced = Vec::new();
    for t in &touching {
        let Kept::Mapped(rel) = &t.kept else { continue };
        let side = |layer: &String| -> Side<'_> {
            match layers.iter().position(|l| l == layer) {
                Some(j) => Side {
                    stored: &items[j].stored,
                    job: Some((j, &inserts[j])),
                },
                None => Side {
                    stored: &partners[layer],
                    job: None,
                },
            }
        };
        let child = side(&t.child_layer);
        // Only the rows of the children this drawing holds: a part's rows outside its area keep theirs.
        let held: std::collections::HashSet<i64> =
            child.stored.rows.values().map(|(f, ..)| *f).collect();
        let rows: Vec<(i64, i64)> = p
            .mapping_rows(&t.child.path, &rel.mapping)
            .map_err(|e| provider_stop(e, &t.child.path, at_one("layer")))?
            .into_iter()
            .filter(|(b, _)| held.contains(b))
            .collect();
        let planned =
            relations::plan_mapping(doc, t, &rel.mapping, &rows, &child, &side(&t.parent_layer))?;
        synced.push(SyncedRelation {
            relation: t.relation.clone(),
            added: planned.added,
            removed: planned.removed,
        });
        if let Some(edit) = planned.edit {
            let mut bases: Vec<gpkg::relations::Key> =
                held.iter().map(|f| gpkg::relations::Key::Fid(*f)).collect();
            bases.sort_by_key(|k| match k {
                gpkg::relations::Key::Fid(f) => *f,
                gpkg::relations::Key::Inserted(..) => i64::MAX,
            });
            maps.push(provider::MapWrite {
                mapping: edit.mapping,
                bases,
                pairs: edit.pairs,
            });
        }
    }
    Ok((maps, synced))
}

/// What is lost and left, said table by table.
fn said(items: &[Item]) -> Vec<CommandWarning> {
    let mut warnings = Vec::new();
    for (i, it) in items.iter().enumerate() {
        let d = &it.diff;
        let mut losses = Vec::new();
        if d.left > 0 {
            losses.push(format!(
                "tablonun geometri tutmadığı {} nesne (yazı, ölçü …) yalnız çizimde kaldı",
                d.left
            ));
        }
        if !d.lost.is_empty() {
            losses.push(format!(
                "katmanın alanı olmayan öznitelikler yazılmadı ({}); Alanlar penceresinden alan yapın",
                d.lost.iter().cloned().collect::<Vec<_>>().join(", ")
            ));
        }
        if !losses.is_empty() {
            warnings.push(CommandWarning {
                code: codes::EXPORT_LOSSES.into(),
                message: format!("{}: {}.", it.link.table, losses.join("; ")),
                path: Some(at(i).into()),
            });
        }
        if it.others && it.base.is_some() {
            warnings.push(CommandWarning {
                code: codes::SOURCE_OTHERS.into(),
                message: format!(
                    "“{}” tablosunda başka birinin değiştirdiği satırlar var; onlara dokunulmadı. Katmana almak için tablodan okuyun (direction read).",
                    it.link.table
                ),
                path: Some(at(i).into()),
            });
        }
        if it.followed.left > 0 {
            warnings.push(CommandWarning {
                code: codes::NEIGHBOURS_LEFT.into(),
                message: format!(
                    "Alanın dışındaki {} komşu kayıt ortak köşeyi izleyince geçersiz kalacağı için olduğu gibi kaldı.",
                    it.followed.left
                ),
                path: Some("topology".into()),
            });
        }
    }
    warnings
}
