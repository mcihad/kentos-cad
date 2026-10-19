//! `cad.sources.relate` v1 (docs/adr/0532 §2): how a project relation's many-to-many reference is kept in the bound
//! GeoPackage, and mapping it to a related-tables relation or taking the mapping out. Nothing is converted behind the
//! user's back: a relation of another representation between the two tables is reported (`foreign`, `broken`) and
//! refused (`relation_unsupported`), never overwritten and never read as semicolon text. The shared cases are
//! `fixtures/commands/v1/cad.sources.relate.json`.

use std::path::Path;

use kentos_contracts::{
    CommandResult, CommandWarning, ProjectRelation, RelateOp, RelationState, SourceKind,
    SourceLink, SourcesRelate, SourcesRelated,
};
use kentos_domain::Document;
use kentos_geopackage::GeoPackage;

use super::relations::{self, Kept, Side};
use super::together::load_link;
use super::{completed, conflict, file_sha, refuse, set_link};
use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;

/// What a check finds of one relation.
struct Found {
    state: RelationState,
    child: Option<SourceLink>,
    parent: Option<SourceLink>,
    kept: Option<Kept>,
}

fn relation<'a>(doc: &'a Document, id: &str) -> Result<&'a ProjectRelation, Stop> {
    doc.settings()
        .relations
        .iter()
        .find(|r| r.id == id)
        .ok_or_else(|| {
            refuse(
                codes::UNKNOWN_RELATION,
                format!("“{id}” kimlikli bir ilişki yok; cad.schema.describe ilişkileri listeler."),
                "relation",
            )
        })
}

fn bound<'a>(doc: &'a Document, layer: &str) -> Option<&'a SourceLink> {
    doc.settings().sources.iter().find(|s| s.layer == layer)
}

/// How the relation's two layers and their file stand.
fn find(doc: &Document, r: &ProjectRelation) -> Result<Found, Stop> {
    let none = |state| Found {
        state,
        child: None,
        parent: None,
        kept: None,
    };
    let (Some(child), Some(parent)) = (bound(doc, &r.child), bound(doc, &r.parent)) else {
        return Ok(none(RelationState::Unbound));
    };
    let (child, parent) = (child.clone(), parent.clone());
    let split = child.kind != SourceKind::Geopackage
        || parent.kind != SourceKind::Geopackage
        || child.path != parent.path
        || r.child == r.parent;
    if split {
        return Ok(Found {
            state: RelationState::Split,
            child: Some(child),
            parent: Some(parent),
            kept: None,
        });
    }
    let path = Path::new(&child.path);
    if !path.is_file() {
        return Err(refuse(
            codes::SOURCE_GONE,
            format!(
                "{} yerinde yok; dosyanın yeni yerini cad.sources.edit relink ile verin.",
                child.path
            ),
            "relation",
        ));
    }
    let single = relations::multiple_field(doc, r).is_none();
    let file = GeoPackage::open(path, false).map_err(|e| {
        refuse(
            codes::IMPORT_UNREADABLE,
            format!("{} GeoPackage olarak okunamadı: {}", child.path, e.0),
            "relation",
        )
    })?;
    let layers = file.layers().map_err(|e| {
        refuse(
            codes::IMPORT_UNREADABLE,
            format!("{} okunamadı: {}", child.path, e.0),
            "relation",
        )
    })?;
    if single {
        return Ok(Found {
            state: RelationState::Single,
            child: Some(child),
            parent: Some(parent),
            kept: None,
        });
    }
    let kept =
        relations::kept_between(&file, &layers, &child.table, &parent.table).map_err(|why| {
            refuse(
                codes::IMPORT_UNREADABLE,
                format!("{} okunamadı: {why}", child.path),
                "relation",
            )
        })?;
    let state = match &kept {
        Kept::Text => RelationState::Text,
        Kept::Mapped(_) => RelationState::Mapped,
        Kept::Foreign(_) => RelationState::Foreign,
        Kept::Broken(_) => RelationState::Broken,
    };
    Ok(Found {
        state,
        child: Some(child),
        parent: Some(parent),
        kept: Some(kept),
    })
}

/// A mapping table's name when none is given: the child's table and the parent's.
fn default_mapping(child: &str, parent: &str) -> String {
    let clean = |s: &str| -> String {
        s.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    };
    clean(&format!("{child}_{parent}"))
        .chars()
        .take(64)
        .collect()
}

/// Why a name is not one a mapping table can have, if it is not.
fn name_problem(name: &str) -> Option<&'static str> {
    let mut chars = name.chars();
    let first = chars.next()?;
    if name.chars().count() > 64 {
        return Some("en çok 64 karakter olur");
    }
    if !(first.is_ascii_alphabetic() || first == '_')
        || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Some("harfle ya da _ ile başlamalı, yalnız harf, rakam ve _ taşımalı");
    }
    let low = name.to_ascii_lowercase();
    if low.starts_with("gpkg") || low.starts_with("sqlite_") || low.starts_with("rtree_") {
        return Some("GeoPackage'ın ve SQLite'ın kendi tablo adlarıyla başlayamaz");
    }
    None
}

fn answer(
    doc: &Document,
    r: &ProjectRelation,
    f: &Found,
    state: RelationState,
    mapping: Option<String>,
    rows: u32,
    source: Option<String>,
) -> SourcesRelated {
    let why = match &f.kept {
        Some(Kept::Foreign(why) | Kept::Broken(why))
            if matches!(state, RelationState::Foreign | RelationState::Broken) =>
        {
            Some(why.clone())
        }
        _ => None,
    };
    SourcesRelated {
        relation: r.id.clone(),
        state,
        mapping,
        rows,
        child: f.child.as_ref().map(|l| l.table.clone()),
        parent: f.parent.as_ref().map(|l| l.table.clone()),
        why,
        source,
        revision: doc.revision().to_string(),
    }
}

fn run(
    doc: &mut Document,
    input: &SourcesRelate,
    write: bool,
) -> Result<(SourcesRelated, Vec<CommandWarning>), Stop> {
    let r = relation(doc, &input.relation)?.clone();
    let f = find(doc, &r)?;
    let (child, parent) = match (&f.child, &f.parent) {
        (Some(c), Some(p)) => (c.clone(), p.clone()),
        _ => {
            if input.action == RelateOp::Check {
                return Ok((answer(doc, &r, &f, f.state, None, 0, None), Vec::new()));
            }
            return Err(refuse(
                codes::RELATION_UNBOUND,
                format!(
                    "“{}” ilişkisinin üst ({}) ya da alt ({}) katmanı bir kaynağa bağlı değil; önce cad.sources.edit ile bağlayın.",
                    r.name, r.parent, r.child
                ),
                "relation",
            ));
        }
    };
    if f.state == RelationState::Split {
        if input.action == RelateOp::Check {
            return Ok((answer(doc, &r, &f, f.state, None, 0, None), Vec::new()));
        }
        return Err(refuse(
            codes::TRANSACTION_SPLIT,
            format!(
                "“{}” ilişkisinin katmanları tek GeoPackage dosyasında değil ({} ve {}); eşleme iki tabloyu tek işlemde ister.",
                r.name, child.path, parent.path
            ),
            "relation",
        ));
    }
    let mapped_rows = |f: &Found| -> Result<(Option<String>, u32), Stop> {
        let Some(Kept::Mapped(rel)) = &f.kept else {
            return Ok((None, 0));
        };
        let file = GeoPackage::open(Path::new(&child.path), false)
            .map_err(|e| refuse(codes::IMPORT_UNREADABLE, e.0, "relation"))?;
        let n = file
            .mapping_rows(&rel.mapping)
            .map_err(|e| refuse(codes::IMPORT_UNREADABLE, e.0, "relation"))?
            .len();
        Ok((Some(rel.mapping.clone()), n as u32))
    };
    if input.action == RelateOp::Check {
        let (mapping, rows) = mapped_rows(&f)?;
        let source = file_sha(Path::new(&child.path)).ok();
        return Ok((
            answer(doc, &r, &f, f.state, mapping, rows, source),
            Vec::new(),
        ));
    }
    if f.state == RelationState::Single {
        return Err(match input.action {
            RelateOp::Map => refuse(
                codes::RELATION_NOT_MULTIPLE,
                format!(
                    "“{}” ilişkisinin ‘{}’ alanı tek kimlik tutuyor (çok değerli değil); eşlenecek bir çoktan çoğa başvuru yok. Alanı Alanlar penceresinden çok değerli yapın.",
                    r.name, r.field
                ),
                "relation",
            ),
            _ => refuse(
                codes::RELATION_NOT_MAPPED,
                format!(
                    "“{}” ilişkisi dosyada eşli değil; kaldırılacak bir eşleme yok.",
                    r.name
                ),
                "relation",
            ),
        });
    }
    // map and unmap write the file: both tables writable, the file as the layers last matched it.
    let (child_stored, _) = load_link(doc, &child, "relation")?;
    for (link, stored_writable) in [(&child, child_stored.writable), (&parent, true)] {
        if !link.writable || !stored_writable {
            return Err(refuse(
                codes::SOURCE_READONLY,
                format!(
                    "{} dosyasının “{}” tablosuna yazılamaz (salt okunur).",
                    link.path, link.table
                ),
                "relation",
            ));
        }
    }
    let sha = child_stored.sha256.clone();
    if child.revision != sha || parent.revision != sha {
        return Err(conflict(
            codes::SOURCE_CHANGED,
            format!(
                "{} katmanlar onunla son eşleştiğinden beri değişti; hiçbir şey yazılmadı. Önce tablodan okuyun (cad.sources.sync read).",
                child.path
            ),
            "relation",
        ));
    }
    match input.action {
        RelateOp::Check => unreachable!("checked above"),
        RelateOp::Map => {
            match &f.kept {
                Some(Kept::Mapped(_)) => {
                    return Err(refuse(
                        codes::RELATION_MAPPED,
                        format!("“{}” ilişkisi dosyada zaten eşli.", r.name),
                        "relation",
                    ));
                }
                Some(Kept::Foreign(why) | Kept::Broken(why)) => {
                    return Err(refuse(
                        codes::RELATION_UNSUPPORTED,
                        format!(
                            "{} ile {} tabloları dosyada başka bir temsille ilişkili: {why}. KentOS onun üzerine yazmaz ve metne çevirmez.",
                            child.table, parent.table
                        ),
                        "relation",
                    ));
                }
                _ => {}
            }
            let name = input
                .mapping
                .clone()
                .unwrap_or_else(|| default_mapping(&child.table, &parent.table));
            if let Some(why) = name_problem(&name) {
                return Err(refuse(
                    codes::INVALID_MAPPING,
                    format!("“{name}” bir eşleme tablosu adı olamaz: {why}."),
                    "mapping",
                ));
            }
            let file = GeoPackage::open(Path::new(&child.path), false)
                .map_err(|e| refuse(codes::IMPORT_UNREADABLE, e.0, "relation"))?;
            if file
                .table_exists(&name)
                .map_err(|e| refuse(codes::IMPORT_UNREADABLE, e.0, "relation"))?
            {
                return Err(refuse(
                    codes::INVALID_MAPPING,
                    format!("Dosyada “{name}” adlı bir tablo zaten var; başka bir ad verin."),
                    "mapping",
                ));
            }
            drop(file);
            let (parent_stored, _) = load_link(doc, &parent, "relation")?;
            let tie = relations::Tie {
                relation: r.id.clone(),
                child_layer: r.child.clone(),
                parent_layer: r.parent.clone(),
                field: relations::multiple_field(doc, &r)
                    .map(|f| f.name.clone())
                    .unwrap_or_else(|| r.field.clone()),
                child: child.clone(),
                parent: parent.clone(),
                kept: Kept::Text,
            };
            let planned = relations::plan_mapping(
                doc,
                &tie,
                &name,
                &[],
                &Side {
                    stored: &child_stored,
                    job: None,
                },
                &Side {
                    stored: &parent_stored,
                    job: None,
                },
            )?;
            let pairs: Vec<(i64, i64)> = planned
                .edit
                .map(|e| {
                    e.pairs
                        .iter()
                        .filter_map(|(b, p)| match (b, p) {
                            (
                                kentos_geopackage::relations::Key::Fid(b),
                                kentos_geopackage::relations::Key::Fid(p),
                            ) => Some((*b, *p)),
                            _ => None,
                        })
                        .collect()
                })
                .unwrap_or_default();
            let rows = pairs.len() as u32;
            if !write {
                return Ok((
                    answer(
                        doc,
                        &r,
                        &f,
                        RelationState::Mapped,
                        Some(name),
                        rows,
                        Some(sha),
                    ),
                    Vec::new(),
                ));
            }
            let mut file = GeoPackage::open(Path::new(&child.path), true).map_err(|e| {
                refuse(
                    codes::FILE_UNWRITABLE,
                    format!("{} yazmak için açılamadı: {}", child.path, e.0),
                    "relation",
                )
            })?;
            file.add_relation(&child_stored.layer, &parent_stored.layer, &name, &pairs)
                .map_err(|e| {
                    refuse(
                        codes::FILE_UNWRITABLE,
                        format!("{} yazılamadı, dosya değişmedi: {}", child.path, e.0),
                        "relation",
                    )
                })?;
            file.close().map_err(|e| {
                refuse(
                    codes::FILE_UNWRITABLE,
                    format!("{} kapatılamadı: {}", child.path, e.0),
                    "relation",
                )
            })?;
            let after = rebind(doc, &child.path, &sha)?;
            Ok((
                answer(
                    doc,
                    &r,
                    &f,
                    RelationState::Mapped,
                    Some(name),
                    rows,
                    Some(after),
                ),
                Vec::new(),
            ))
        }
        RelateOp::Unmap => {
            let Some(Kept::Mapped(rel)) = &f.kept else {
                return Err(refuse(
                    codes::RELATION_NOT_MAPPED,
                    format!(
                        "“{}” ilişkisi dosyada eşli değil; kaldırılacak bir eşleme yok.",
                        r.name
                    ),
                    "relation",
                ));
            };
            if !write {
                return Ok((
                    answer(doc, &r, &f, RelationState::Text, None, 0, Some(sha)),
                    Vec::new(),
                ));
            }
            let mut file = GeoPackage::open(Path::new(&child.path), true).map_err(|e| {
                refuse(
                    codes::FILE_UNWRITABLE,
                    format!("{} yazmak için açılamadı: {}", child.path, e.0),
                    "relation",
                )
            })?;
            file.drop_relation(rel).map_err(|e| {
                refuse(
                    codes::FILE_UNWRITABLE,
                    format!("{} yazılamadı, dosya değişmedi: {}", child.path, e.0),
                    "relation",
                )
            })?;
            file.close().map_err(|e| {
                refuse(
                    codes::FILE_UNWRITABLE,
                    format!("{} kapatılamadı: {}", child.path, e.0),
                    "relation",
                )
            })?;
            let after = rebind(doc, &child.path, &sha)?;
            Ok((
                answer(doc, &r, &f, RelationState::Text, None, 0, Some(after)),
                Vec::new(),
            ))
        }
    }
}

/// The file's new SHA-256 into every bound table of it that matched the old one: the tables the write did not touch
/// still match the file.
fn rebind(doc: &mut Document, path: &str, before: &str) -> Result<String, Stop> {
    let after =
        file_sha(Path::new(path)).map_err(|why| refuse(codes::FILE_UNWRITABLE, why, "relation"))?;
    let links: Vec<SourceLink> = doc
        .settings()
        .sources
        .iter()
        .filter(|s| s.path == path && s.revision == before)
        .cloned()
        .collect();
    for mut link in links {
        link.revision = after.clone();
        set_link(doc, Some(link), None);
    }
    Ok(after)
}

pub fn validate(cx: &ExecutionContext<'_>, input: &SourcesRelate) -> CommandResult<()> {
    let mut copy = cx.doc.clone();
    match run(&mut copy, input, false) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &SourcesRelate) -> CommandResult<SourcesRelated> {
    let mut copy = cx.doc.clone();
    completed(run(&mut copy, input, false))
}

pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: SourcesRelate,
) -> CommandResult<SourcesRelated> {
    // A plan through the protocol runs this on a reading copy: it writes no file then (docs/adr/0532 §4).
    let write = !cx.dry();
    completed(run(cx.doc, &input, write))
}
