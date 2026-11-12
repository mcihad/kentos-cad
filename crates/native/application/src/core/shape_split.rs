//! `cad.shapes.edit`'s Parçala (docs/adr/0140, 0622 §9): lines, open polylines, arcs and circles cut where the given
//! objects cross one another, into equal parts, or into pieces of a length, with the shared core
//! (`ops::split`). The first piece keeps the object's place, id and data, the others are new objects from it; the
//! shared cases are `scripts/fixtures/split_pieces_cases.py`'s.

use kentos_commands::{Entity, EntityEdit, SplitMethod};
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape};
use kentos_geometry_core::ops::path::path_of;
use kentos_geometry_core::ops::split::{split_at_crossings, split_by_length, split_equal};

use crate::checks::{self, Stop, error};
use crate::codes;
use crate::geometry::{edit_geometry, shape};

/// Most pieces one object may come apart into.
const MOST_PIECES: f64 = 10_000.0;

fn refuse(code: &str, message: &str, path: &str) -> Stop {
    Stop::failed(error(code, message.to_owned(), Some(path.into())))
}

/// What Parçala writes: the edits, the objects cut, those left whole and the pieces.
pub(crate) struct Cut {
    pub changes: Vec<EntityEdit>,
    pub objects: usize,
    pub skipped: usize,
    pub pieces: usize,
}

/// The kinds Parçala cuts, as the core's `splittable` (a multi-part polyline is not).
fn cuttable(e: &Entity) -> bool {
    matches!(
        e,
        Entity::Line(_) | Entity::Polyline(_) | Entity::Arc(_) | Entity::Circle(_)
    ) && !kentos_geometry_core::entity::is_multi_part(&shape(e))
}

/// The objects as given (`uid`, entity), each once.
pub(crate) fn split(
    list: &[(String, &Entity)],
    method: SplitMethod,
    parts: Option<u32>,
    length: Option<f64>,
    from_end: bool,
) -> Result<Cut, Stop> {
    // The numbers first, then what the objects are.
    match method {
        SplitMethod::Crossings => {}
        SplitMethod::Equal => {
            if !parts.is_some_and(|n| (2..=10_000).contains(&n)) {
                return Err(refuse(
                    codes::INVALID_COUNT,
                    "Parça sayısı 2 ile 10 000 arasında olmalı.",
                    "work.parts",
                ));
            }
        }
        SplitMethod::Length => {
            if let Some(l) = length {
                checks::finite(l, "Uzunluk", "Sonlu bir sayı verin.", "work.length")?;
            }
            if !length.is_some_and(|l| l > 0.0) {
                return Err(refuse(
                    codes::INVALID_DISTANCE,
                    "Parça uzunluğu sıfırdan büyük olmalı.",
                    "work.length",
                ));
            }
        }
    }
    let given = list.len();
    let cut: Vec<(String, Vec<Shape>)> = match method {
        SplitMethod::Crossings => {
            if given < 2 {
                return Err(refuse(
                    codes::TOO_FEW_OBJECTS,
                    "Kesişimlerden bölmek için en az iki nesne verin.",
                    "work.ids",
                ));
            }
            if !list.iter().any(|(_, e)| cuttable(e)) {
                return Err(not_cut());
            }
            let entities: Vec<CoreEntity> = list
                .iter()
                .map(|(_, e)| CoreEntity::new(shape(e)))
                .collect();
            let found: Vec<(String, Vec<Shape>)> = split_at_crossings(&entities)
                .into_iter()
                .map(|p| {
                    (
                        list[p.index].0.clone(),
                        p.pieces.into_iter().map(|e| e.shape).collect(),
                    )
                })
                .collect();
            if found.is_empty() {
                return Err(refuse(
                    codes::NO_INTERSECTION,
                    "Verilen nesneler birbirini kesmiyor; bölünecek yer yok.",
                    "work.ids",
                ));
            }
            found
        }
        SplitMethod::Equal | SplitMethod::Length => {
            let takers: Vec<&(String, &Entity)> =
                list.iter().filter(|(_, e)| cuttable(e)).collect();
            if takers.is_empty() {
                return Err(not_cut());
            }
            let mut found = Vec::new();
            for (uid, e) in takers {
                let core = CoreEntity::new(shape(e));
                let pieces = if method == SplitMethod::Equal {
                    split_equal(&core, f64::from(parts.unwrap_or(0)))
                } else {
                    let l = length.unwrap_or(0.0);
                    if path_of(&core.shape).is_some_and(|p| p.length / l > MOST_PIECES) {
                        return Err(refuse(
                            codes::TOO_MANY_POINTS,
                            "Bu uzunlukla 10 000'den çok parça olur; uzunluğu artırın.",
                            "work.length",
                        ));
                    }
                    split_by_length(&core, l, from_end)
                };
                if let Some(pieces) = pieces {
                    found.push((uid.clone(), pieces.into_iter().map(|e| e.shape).collect()));
                }
            }
            if found.is_empty() {
                return Err(refuse(
                    codes::INVALID_DISTANCE,
                    "Parça uzunluğu nesnenin boyundan kısa olmalı; hiçbir nesne bölünmedi.",
                    "work.length",
                ));
            }
            found
        }
    };
    let mut changes = Vec::new();
    let mut pieces = 0;
    for (uid, shapes) in &cut {
        let mut geometries = shapes.iter().cloned().filter_map(edit_geometry);
        let Some(first) = geometries.next() else {
            continue;
        };
        pieces += 1;
        changes.push(EntityEdit::Replace {
            uid: uid.clone(),
            geometry: first,
            keep_data: Some(true),
        });
        for geometry in geometries {
            pieces += 1;
            changes.push(EntityEdit::add(uid.clone(), geometry, Some(true)));
        }
    }
    Ok(Cut {
        changes,
        objects: cut.len(),
        skipped: given - cut.len(),
        pieces,
    })
}

fn not_cut() -> Stop {
    refuse(
        codes::NOT_A_PATH,
        "Bölünecek çizgi, açık çoklu çizgi, yay ya da daire yok; alanları önce Patlat ile çizgilere ayırın.",
        "work.ids",
    )
}
