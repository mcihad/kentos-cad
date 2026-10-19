//! `cad.assets.check` v1 (docs/adr/0349 §2; plan/cad.md `CAD-REF-19`): every file the drawing links to, where it was
//! found and whether it changed since it was read. Read-only. External references, linked images and rasters are looked
//! for with the project's search paths (`links::find`, docs/adr/0350 §2), the other links at their path and from the
//! drawing's folder, as their own loaders do.
//! The shared cases are `fixtures/commands/v1/cad.assets.check.json`.

use std::path::Path;

use kentos_contracts::encoding::sha256_reader_hex;
use kentos_contracts::{
    AssetCounts, AssetKind, AssetState, AssetStatus, AssetVia, AssetsCheck, AssetsChecked, BlockId, CommandResult,
    Entity, SourceKind, TableSource,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::links::{Found, find};

fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned())
}

/// What a check gathers, in the answer's order.
struct Walk<'a> {
    input: &'a AssetsCheck,
    /// The project's search paths: external references', images' and rasters' (docs/adr/0350 §2).
    search: Vec<String>,
    out: Vec<AssetStatus>,
}

impl Walk<'_> {
    fn wants(&self, kind: AssetKind) -> bool {
        self.input.kinds.as_ref().is_none_or(|k| k.contains(&kind))
    }

    fn digests(&self) -> bool {
        self.input.digests.unwrap_or(true)
    }

    /// A linked file at `path` (absolute, or relative to the drawing's folder), its SHA-256 kept as `digest`.
    #[allow(clippy::too_many_arguments)]
    fn file(
        &mut self,
        kind: AssetKind,
        path: &str,
        relative: Option<&str>,
        search: &[String],
        digest: Option<&str>,
        uid: Option<String>,
        block: Option<BlockId>,
        source: Option<String>,
        name: Option<String>,
    ) {
        if !self.wants(kind) {
            return;
        }
        let Found { at, via, tried } = find(path, relative, self.input.drawing.as_deref(), search);
        let (state, current) = match (&at, digest) {
            (None, _) => (AssetState::Missing, None),
            (Some(p), Some(kept)) if self.digests() => match std::fs::File::open(p).and_then(sha256_reader_hex) {
                Ok((_, now)) if now == kept => (AssetState::Ok, None),
                Ok((_, now)) => (AssetState::Changed, Some(now)),
                Err(_) => (AssetState::Missing, None),
            },
            (Some(_), _) => (AssetState::Ok, None),
        };
        self.out.push(AssetStatus {
            kind,
            uid,
            block,
            source,
            name: name.unwrap_or_else(|| file_name(path)),
            path: path.to_owned(),
            relative: relative.map(str::to_owned),
            found: at.map(|p| p.display().to_string()),
            via,
            state,
            digest: digest.map(str::to_owned),
            current,
            tried,
        });
    }

    /// An address, not looked at; called `name`, else its last part.
    fn address(&mut self, kind: AssetKind, url: &str, uid: Option<String>, block: Option<BlockId>, name: Option<String>) {
        if !self.wants(kind) {
            return;
        }
        self.out.push(AssetStatus {
            kind,
            uid,
            block,
            source: None,
            name: name.unwrap_or_else(|| file_name(url.split(['?', '#']).next().unwrap_or(url))),
            path: url.to_owned(),
            relative: None,
            found: None,
            via: AssetVia::Address,
            state: AssetState::Remote,
            digest: None,
            current: None,
            tried: Vec::new(),
        });
    }

    /// One object's links: its own file, then its attachments.
    fn entity(&mut self, e: &Entity, uid: Option<String>, block: Option<BlockId>) {
        match e {
            Entity::Image(i) => {
                if let Some(f) = &i.image.file {
                    let search = self.search.clone();
                    self.file(AssetKind::Image, f, None, &search, None, uid.clone(), block, None, None);
                }
            }
            Entity::Raster(r) => {
                if let Some(f) = &r.raster.file {
                    let search = self.search.clone();
                    self.file(AssetKind::Raster, f, None, &search, None, uid.clone(), block, None, None);
                }
                if let Some(u) = &r.raster.url {
                    self.address(AssetKind::Raster, u, uid.clone(), block, None);
                }
            }
            Entity::PointCloud(c) => {
                for s in &c.cloud.sources {
                    if let Some(f) = &s.file {
                        self.file(AssetKind::PointCloud, f, None, &[], None, uid.clone(), block, None, None);
                    }
                    if let Some(u) = &s.url {
                        self.address(AssetKind::PointCloud, u, uid.clone(), block, None);
                    }
                }
            }
            Entity::Table(t) => {
                if let Some(TableSource::File { path: Some(p), digest, .. }) = &t.source {
                    self.file(AssetKind::Table, p, None, &[], digest.as_deref(), uid.clone(), block, None, None);
                }
            }
            _ => {}
        }
        for a in &e.base().attachments {
            let name = (!a.name.trim().is_empty()).then(|| a.name.clone());
            if let Some(f) = &a.file {
                self.file(AssetKind::Attachment, f, None, &[], None, uid.clone(), block, None, name.clone());
            }
            if let Some(u) = &a.url {
                self.address(AssetKind::Attachment, u, uid.clone(), block, name);
            }
        }
    }
}

/// The drawing's linked files.
pub fn check_in(doc: &Document, input: &AssetsCheck) -> AssetsChecked {
    let search = doc.settings().xref_paths.clone();
    let mut w = Walk { input, search: search.clone(), out: Vec::new() };
    // External references, in the drawing's block order.
    for b in doc.blocks() {
        let Some(x) = &b.xref else { continue };
        if !w.wants(AssetKind::Xref) {
            break;
        }
        if x.unloaded {
            w.out.push(AssetStatus {
                kind: AssetKind::Xref,
                uid: None,
                block: Some(b.id),
                source: None,
                name: b.name.clone(),
                path: x.path.clone(),
                relative: x.relative.clone(),
                found: None,
                via: AssetVia::None,
                state: AssetState::Unloaded,
                digest: Some(x.digest.clone()),
                current: None,
                tried: Vec::new(),
            });
            continue;
        }
        w.file(
            AssetKind::Xref,
            &x.path,
            x.relative.as_deref(),
            &search,
            Some(&x.digest),
            None,
            Some(b.id),
            None,
            Some(b.name.clone()),
        );
    }
    // The drawing's objects, then the blocks' (an external reference's content is its source's, not linked again).
    for e in doc.entities() {
        let uid = doc.uid(Slot(e.base().id)).map(|u| u.to_string());
        w.entity(e, uid, None);
    }
    for b in doc.blocks() {
        if b.xref.is_some() {
            continue;
        }
        for e in &b.entities {
            w.entity(e, None, Some(b.id));
        }
    }
    // Bound layers' sources.
    if w.wants(AssetKind::Source) {
        for s in &doc.settings().sources {
            if s.kind == SourceKind::Postgis {
                w.out.push(AssetStatus {
                    kind: AssetKind::Source,
                    uid: None,
                    block: None,
                    source: Some(s.id.clone()),
                    name: s.table.clone(),
                    path: s.path.clone(),
                    relative: None,
                    found: None,
                    via: AssetVia::Database,
                    state: AssetState::Database,
                    digest: None,
                    current: None,
                    tried: Vec::new(),
                });
                continue;
            }
            w.file(
                AssetKind::Source,
                &s.path,
                s.relative.as_deref(),
                &[],
                Some(&s.revision),
                None,
                None,
                Some(s.id.clone()),
                None,
            );
        }
    }
    let mut counts = AssetCounts::default();
    for a in &w.out {
        match a.state {
            AssetState::Ok => counts.ok += 1,
            AssetState::Changed => counts.changed += 1,
            AssetState::Missing => counts.missing += 1,
            _ => counts.other += 1,
        }
    }
    AssetsChecked { assets: w.out, counts, revision: doc.revision().to_string() }
}

pub fn validate(_cx: &ExecutionContext<'_>, _input: &AssetsCheck) -> CommandResult<()> {
    CommandResult::Completed { output: (), warnings: Vec::new() }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &AssetsCheck) -> CommandResult<AssetsChecked> {
    CommandResult::Completed { output: check_in(cx.doc, input), warnings: Vec::new() }
}

/// Read-only: the same answer as the plan.
pub fn execute(cx: &mut ExecutionContext<'_>, input: AssetsCheck) -> CommandResult<AssetsChecked> {
    plan(cx, &input)
}
