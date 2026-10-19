//! Bound layers written to one GeoPackage file in one transaction (docs/adr/0532 §1). A parent and its children, or a
//! layer and the mapping table of a many-to-many reference it holds, go in together or not at all: every layer is
//! checked first (read-only table, file changed since, geometry the column cannot hold, a reference to an object the
//! table lacks), the edits of all of them and of the mapping tables are made, then written with
//! `GeoPackage::edit_many` in one SQLite transaction. A single layer takes the same road, so a mapped reference
//! follows it wherever it is written.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use kentos_contracts::{
    CommandWarning, SourceLink, SourcesSynced, SyncTopology, SyncedLayer, SyncedRelation,
};
use kentos_domain::Document;
use kentos_geopackage::{Edits, GeoPackage};

use super::relations::{self, Kept, Side, Tie};
use super::{
    Diff, Followed, Mover, Origin, Stored, at_one, codes, conflict, derived_message, diff_skipping,
    file_sha, follow_outside, layer_digest, load_at, mover_for, origin_from, refuse, set_link,
    table_area,
};
use crate::checks::Stop;

/// A layer of the write: its bond, its table as the file holds it and how it differs.
struct Item {
    link: SourceLink,
    stored: Stored,
    diff: Diff,
    /// The rows outside a part's area that follow the layer's shared corners (docs/adr/0531).
    followed: Followed,
}

/// Where a refusal points: `layer` for the first layer, `with` for the others.
fn at(i: usize) -> &'static str {
    if i == 0 { "layer" } else { "with" }
}

/// A bound layer's table as a sync reads it, with the move and the part's area (in the table's system) it was read by.
pub(super) struct Loaded {
    pub stored: Stored,
    pub mover: Option<Mover>,
    pub area: Option<[f64; 4]>,
}

/// A bound layer's table, read as a sync reads it: in the table's system, the rows the move reaches.
pub(super) fn load_link(doc: &Document, link: &SourceLink, at: &str) -> Result<Loaded, Stop> {
    let mover = mover_for(doc, link.srid).map_err(|why| {
        refuse(
            codes::CRS_MISMATCH,
            format!("“{}” eşitlenemez: {why}.", link.layer),
            at,
        )
    })?;
    let area = table_area(link.extent, mover.as_ref()).map_err(|()| {
        refuse(
            codes::INVALID_EXTENT,
            format!(
                "“{}” katmanının alan sınırı tablonun sistemine çevrilemiyor.",
                link.layer
            ),
            at,
        )
    })?;
    let mut stored = load_at(
        origin_from(link.kind, &link.path),
        &link.table,
        at_one(at),
        area,
    )?;
    stored.reachable(mover.as_ref());
    Ok(Loaded {
        stored,
        mover,
        area,
    })
}

/// The names of the child fields whose references a mapping table holds, among `ties` for `layer`.
fn skipped(ties: &[Tie], layer: &str) -> Vec<String> {
    ties.iter()
        .filter(|t| matches!(t.kept, Kept::Mapped(_)) && t.child_layer == layer)
        .map(|t| t.field.clone())
        .collect()
}

/// Sets the many-to-many references of a bound child layer's objects from the mapping tables KentOS made for them: the
/// parents' lasting ids in the rows' order, the attribute taken off when a child has none.
pub(super) fn patch_read(
    doc: &Document,
    link: &SourceLink,
    stored: &mut Stored,
) -> Result<(), Stop> {
    let ties = relations::ties(doc);
    for t in ties
        .iter()
        .filter(|t| t.child_layer == link.layer && matches!(t.kept, Kept::Mapped(_)))
    {
        let Kept::Mapped(rel) = &t.kept else { continue };
        let file = GeoPackage::open(Path::new(&link.path), false).map_err(|e| {
            refuse(
                codes::IMPORT_UNREADABLE,
                format!("{} açılamadı: {}", link.path, e.0),
                "layer",
            )
        })?;
        let broken = |e: kentos_geopackage::Error| {
            refuse(
                codes::IMPORT_UNREADABLE,
                format!("“{}” ilişkisi okunamadı: {}", rel.mapping, e.0),
                "layer",
            )
        };
        let layers = file.layers().map_err(broken)?;
        let Some(parent) = layers.iter().find(|l| l.name == t.parent.table) else {
            continue;
        };
        let rows = file.mapping_rows(&rel.mapping).map_err(broken)?;
        let fids: HashSet<i64> = file.fids(parent).map_err(broken)?.into_iter().collect();
        let uids = file.uids(&parent.name).map_err(broken)?;
        let refs = relations::read_references(&rows, &parent.name, &fids, &uids, &rel.mapping)?;
        for (uid, e) in &mut stored.objects {
            let Some((fid, ..)) = stored.rows.get(uid) else {
                continue;
            };
            match refs.get(fid) {
                Some(list) => {
                    e.base_mut().attrs.insert(t.field.clone(), list.join("; "));
                }
                None => {
                    e.base_mut().attrs.remove(&t.field);
                }
            }
        }
    }
    Ok(())
}

/// Writes the layers `layers` (the first is the one asked, the rest `with`) to their file in one transaction; with
/// `write` false, only says what it would do.
pub(super) fn write(
    doc: &mut Document,
    layers: &[String],
    write: bool,
    topology: Option<SyncTopology>,
) -> Result<(SourcesSynced, Vec<CommandWarning>), Stop> {
    let mut seen = HashSet::new();
    for (i, l) in layers.iter().enumerate() {
        if !seen.insert(l.as_str()) {
            return Err(refuse(
                codes::INVALID_INPUT,
                format!("“{l}” katmanı birden çok kez verildi; her katman bir kez yazılır."),
                at(i),
            ));
        }
    }
    // Every layer bound, its file the same, then read.
    let mut links: Vec<SourceLink> = Vec::with_capacity(layers.len());
    for (i, id) in layers.iter().enumerate() {
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
    let path = links[0].path.clone();
    for (i, l) in links.iter().enumerate() {
        if l.kind != kentos_contracts::SourceKind::Geopackage || l.path != path {
            return Err(refuse(
                codes::TRANSACTION_SPLIT,
                format!(
                    "“{}” katmanı {} kaynağında, “{}” katmanı {path} dosyasında; birlikte yazılacak katmanlar tek GeoPackage dosyasında olmalı. Katmanları ayrı ayrı yazın.",
                    l.layer, l.path, links[0].layer
                ),
                at(i),
            ));
        }
        if !Path::new(&l.path).is_file() {
            return Err(refuse(
                codes::SOURCE_GONE,
                format!(
                    "{} yerinde yok; dosyanın yeni yerini cad.sources.edit relink ile verin.",
                    l.path
                ),
                at(i),
            ));
        }
        // A derivative of a drawing is never written back (docs/adr/0530 §4).
        if let Some(rec) = crate::gis_derive::derived_table(Path::new(&l.path), &l.table) {
            return Err(refuse(
                codes::DERIVED_DATASET,
                derived_message(&l.table, &rec),
                at(i),
            ));
        }
    }
    let ties = relations::ties(doc);
    let mut items: Vec<Item> = Vec::with_capacity(links.len());
    for (i, link) in links.iter().enumerate() {
        let Loaded {
            stored,
            mover,
            area,
        } = load_link(doc, link, at(i))?;
        if !link.writable || !stored.writable {
            return Err(refuse(
                codes::SOURCE_READONLY,
                format!(
                    "{} dosyasının “{}” tablosuna yazılamaz (salt okunur).",
                    link.path, link.table
                ),
                at(i),
            ));
        }
        if stored.sha256 != link.revision {
            return Err(conflict(
                codes::SOURCE_CHANGED,
                format!(
                    "{} katman onunla son eşleştiğinden beri değişti; üzerine yazılmadı. Önce tablodan okuyun (direction read) ya da dosyanın kopyasını bağlayın.",
                    link.path
                ),
                at(i),
            ));
        }
        let d = diff_skipping(
            doc,
            &link.layer,
            &stored,
            mover.as_ref(),
            &skipped(&ties, &link.layer),
        );
        if let Some((uid, why)) = &d.mismatch {
            return Err(refuse(
                codes::GEOMETRY_MISMATCH,
                format!("{uid} kimlikli nesne tabloya yazılamaz: {why}."),
                at(i),
            ));
        }
        // The neighbours outside a part's area follow the written objects' shared corners and edges (docs/adr/0531).
        let followed = match topology {
            None => Followed::default(),
            Some(profile) => {
                let mut columns = stored.layer.columns.clone();
                columns.extend(d.added_columns.iter().cloned());
                follow_outside(
                    doc,
                    link,
                    Origin::File(Path::new(&link.path)),
                    area,
                    &stored,
                    &columns,
                    mover.as_ref(),
                    profile,
                )?
            }
        };
        items.push(Item {
            link: link.clone(),
            stored,
            diff: d,
            followed,
        });
    }
    // Mapped references that touch the layers: their mapping rows follow the layers' references.
    let touching = relations::mapped_touching(&ties, layers);
    let file = GeoPackage::open(Path::new(&path), false).map_err(|e| {
        refuse(
            codes::FILE_UNWRITABLE,
            format!("{path} açılamadı: {}", e.0),
            "layer",
        )
    })?;
    let inserts: Vec<HashMap<kentos_domain::Uuid, usize>> = items
        .iter()
        .map(|it| {
            it.diff
                .insert
                .iter()
                .enumerate()
                .filter_map(|(i, (u, _))| kentos_domain::Uuid::parse_str(u).ok().map(|u| (u, i)))
                .collect()
        })
        .collect();
    let mut partners: HashMap<String, Stored> = HashMap::new();
    for t in &touching {
        for (layer, link) in [(&t.child_layer, &t.child), (&t.parent_layer, &t.parent)] {
            if !layers.contains(layer) && !partners.contains_key(layer) {
                let loaded = load_link(doc, link, "layer")?;
                partners.insert(layer.clone(), loaded.stored);
            }
        }
    }
    let mut maps = Vec::new();
    let mut synced_relations = Vec::new();
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
        let rows = file.mapping_rows(&rel.mapping).map_err(|e| {
            refuse(
                codes::IMPORT_UNREADABLE,
                format!("“{}” eşleme tablosu okunamadı: {}", rel.mapping, e.0),
                "layer",
            )
        })?;
        let planned = relations::plan_mapping(
            doc,
            t,
            &rel.mapping,
            &rows,
            &side(&t.child_layer),
            &side(&t.parent_layer),
        )?;
        synced_relations.push(SyncedRelation {
            relation: t.relation.clone(),
            added: planned.added,
            removed: planned.removed,
        });
        maps.extend(planned.edit);
    }
    drop(file);
    // What is lost, said table by table.
    let mut warnings = Vec::new();
    for it in &items {
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
                path: Some("layer".into()),
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
    let mut columns: Vec<String> = Vec::new();
    for it in &items {
        columns.extend(it.diff.added_columns.iter().map(|c| c.name.clone()));
    }
    let mut out = SourcesSynced {
        added: layers_out.iter().map(|l| l.added).sum(),
        modified: layers_out.iter().map(|l| l.modified).sum(),
        removed: layers_out.iter().map(|l| l.removed).sum(),
        columns,
        source: items[0].stored.sha256.clone(),
        revision: doc.revision().to_string(),
        layers: if layers.len() > 1 {
            layers_out
        } else {
            Vec::new()
        },
        relations: synced_relations,
        pins: None,
        followed: items.iter().map(|it| it.followed.rows.len() as u32).sum(),
        left: items.iter().map(|it| it.followed.left).sum(),
    };
    if !write {
        return Ok((out, warnings));
    }
    let before = items[0].stored.sha256.clone();
    let nothing = items.iter().all(|it| {
        it.diff.is_empty() && it.diff.added_columns.is_empty() && it.followed.rows.is_empty()
    }) && maps.is_empty();
    if !nothing {
        let edits: Vec<Edits> = items
            .iter_mut()
            .map(|it| {
                let mut update = std::mem::take(&mut it.diff.update);
                update.extend(std::mem::take(&mut it.followed.rows));
                update.sort_by_key(|(fid, _)| *fid);
                Edits {
                    add_columns: std::mem::take(&mut it.diff.added_columns),
                    update,
                    insert: std::mem::take(&mut it.diff.insert),
                    delete: std::mem::take(&mut it.diff.delete),
                }
            })
            .collect();
        let jobs: Vec<(&kentos_geopackage::Layer, &Edits)> = items
            .iter()
            .zip(&edits)
            .map(|(it, e)| (&it.stored.layer, e))
            .collect();
        let mut file = GeoPackage::open(Path::new(&path), true).map_err(|e| {
            refuse(
                codes::FILE_UNWRITABLE,
                format!("{path} yazmak için açılamadı: {}", e.0),
                "layer",
            )
        })?;
        file.edit_many(&jobs, &maps).map_err(|e| {
            refuse(
                codes::FILE_UNWRITABLE,
                format!("{path} yazılamadı, dosya değişmedi: {}", e.0),
                "layer",
            )
        })?;
        file.close().map_err(|e| {
            refuse(
                codes::FILE_UNWRITABLE,
                format!("{path} kapatılamadı: {}", e.0),
                "layer",
            )
        })?;
    }
    let after =
        file_sha(Path::new(&path)).map_err(|why| refuse(codes::FILE_UNWRITABLE, why, "layer"))?;
    for it in &items {
        let mut link = it.link.clone();
        link.revision = after.clone();
        link.digest = layer_digest(doc, &link.layer);
        set_link(doc, Some(link), None);
    }
    // The file's other bound tables were not touched: they match the file as they did.
    if after != before {
        let others: Vec<SourceLink> = doc
            .settings()
            .sources
            .iter()
            .filter(|s| s.path == path && s.revision == before && !layers.contains(&s.layer))
            .cloned()
            .collect();
        for mut link in others {
            link.revision = after.clone();
            set_link(doc, Some(link), None);
        }
    }
    out.source = after;
    out.revision = doc.revision().to_string();
    Ok((out, warnings))
}
