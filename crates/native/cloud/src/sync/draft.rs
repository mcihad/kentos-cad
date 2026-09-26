//! The device draft of an open database project (docs/adr/0040), as the web
//! keeps it (apps/web/src/app/cloud/drafts.ts, syncRestore.ts; CLAUDE.md
//! §21.3): what is not sent yet, and the command on its way with its
//! idempotency key, written on this device as edits happen, so a crash or a
//! lost connection loses nothing and a command whose answer was lost is
//! answered once. The file's shape is the web's draft, format 2: changes by
//! the object's persistent id (its id on the server), each with the server
//! version it was based on. A draft belongs to one account and one project;
//! it never holds a session or a password (drafts.rs stores it).
//!
//! Putting a draft back into a fresh opening ([`ProjectSync::restore`]):
//!
//! - the command that was on its way goes first, as it was: if it had been
//!   committed the server answers from its log, and its changes are the
//!   server's then. Its changes are in the drawing at once (this device may
//!   have no connection to wait for) and count as unsent until it is answered;
//! - every other change goes into the drawing as unsent local work, without
//!   an undo step; one whose base the server moved past meanwhile is a
//!   conflict (the drawing shows the local copy until the user chooses), and
//!   a deletion the server has too is done;
//! - a change the drawing cannot take (its layer is gone) stays in the next
//!   draft, unsent, and the user is told.

use std::collections::{BTreeMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use kentos_contracts::{
    CommandEnvelope, ConflictReason, Entity, FeatureChange, FeatureRecord, ProjectChanges,
    ProjectPatch,
};
use kentos_domain::{Document, External, ExternalMeta};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::remote::is_layer;
use super::{Conflict, Inflight, Meta, PROJECT_KEY, Planned, ProjectSync, SaveState};

/// The draft format this code writes (the web's `DRAFT_VERSION`).
pub const DRAFT_VERSION: u32 = 2;

/// One object's unsent state: what it should become (`None`: deleted) and
/// the server version it was based on (`None`: the server does not have it).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DraftChange {
    pub base: Option<String>,
    pub entity: Option<Entity>,
}

/// Unsent metadata: the patch and the metadata version it was based on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DraftMeta {
    pub base: String,
    pub patch: ProjectPatch,
}

/// What of one project is on this device and not on the server.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    pub version: u32,
    /// The account it belongs to: another account's draft is never put back.
    pub user_id: String,
    /// By the object's persistent id, lowercase with hyphens.
    pub changes: BTreeMap<String, DraftChange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<DraftMeta>,
    /// The command on its way: sent again with its key when the draft comes back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inflight: Option<CommandEnvelope>,
    /// When it was written, in milliseconds since 1970.
    pub updated: u64,
}

/// What putting a draft back did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Restored {
    /// Objects (and the metadata) that went back into the drawing, unsent.
    pub changed: usize,
    /// Of those, the ones the server moved past meanwhile: now conflicts.
    pub conflicts: usize,
    /// A command was on its way: it is sent again first, with its key.
    pub resends: bool,
    /// Changes kept aside, unsent (the drawing could not take them), with the reason, in Turkish.
    pub held: Vec<String>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// The work a command was planned from, read back from its envelope.
fn planned_of(change: &FeatureChange, expected: &BTreeMap<String, String>) -> Option<Planned> {
    Some(match change {
        FeatureChange::Create { id, entity } => Planned::Create {
            id: Uuid::parse_str(id).ok()?,
            entity: entity.clone(),
        },
        FeatureChange::Update { id, entity } => Planned::Update {
            id: Uuid::parse_str(id).ok()?,
            entity: entity.clone(),
            expected: expected.get(id)?.clone(),
        },
        FeatureChange::Delete { id } => Planned::Delete {
            id: Uuid::parse_str(id).ok()?,
            expected: expected.get(id)?.clone(),
        },
    })
}

impl Meta {
    /// This metadata with a patch applied.
    fn patched(&self, patch: &ProjectPatch) -> Meta {
        Meta {
            name: patch.name.clone().unwrap_or_else(|| self.name.clone()),
            settings: patch
                .settings
                .clone()
                .unwrap_or_else(|| self.settings.clone()),
            layers: patch.layers.clone().unwrap_or_else(|| self.layers.clone()),
            styles: patch.styles.clone().unwrap_or_else(|| self.styles.clone()),
        }
    }
}

impl ProjectSync {
    /// What of this project is not on the server, for the device: `None`
    /// when nothing is (the stored draft is then removed). A viewer's own
    /// edits are never kept (they were told so).
    pub fn draft(&mut self, doc: &Document, user_id: &str) -> Option<Draft> {
        if !self.can_write && !self.state.ended() {
            return None;
        }
        self.observe(doc);
        let mut changes: BTreeMap<String, DraftChange> = self
            .held
            .iter()
            .map(|(id, c)| (id.to_string(), c.clone()))
            .collect();
        for &id in &self.dirty {
            let change = match self.plan(doc, id) {
                Some(Planned::Create { entity, .. }) => DraftChange {
                    base: None,
                    entity: Some(entity),
                },
                Some(Planned::Update {
                    entity, expected, ..
                }) => DraftChange {
                    base: Some(expected),
                    entity: Some(entity),
                },
                Some(Planned::Delete { expected, .. }) => DraftChange {
                    base: Some(expected),
                    entity: None,
                },
                None => continue,
            };
            changes.insert(id.to_string(), change);
        }
        let meta = if self.sends_meta() {
            self.meta_base.patch(doc).map(|patch| DraftMeta {
                base: self.meta_version.clone(),
                patch,
            })
        } else {
            None
        };
        let inflight = self.inflight.as_ref().map(|f| f.envelope.clone());
        if changes.is_empty() && meta.is_none() && inflight.is_none() {
            return None;
        }
        Some(Draft {
            version: DRAFT_VERSION,
            user_id: user_id.to_owned(),
            changes,
            meta,
            inflight,
            updated: now_ms(),
        })
    }

    /// Puts a draft of this device back into the project just opened (see
    /// the module comment). Refused, with nothing changed, while an edit is
    /// open. Call it before any edit of the new opening.
    pub fn restore(&mut self, doc: &mut Document, draft: Draft) -> Result<Restored, String> {
        if doc.is_busy() {
            return Err(
                "Açık bir düzenleme var; cihazdaki taslak o bitince geri konur.".to_owned(),
            );
        }
        self.observe(doc);
        let mut restored = Restored::default();
        // The command on its way goes again first, with its key; its changes are its own.
        let mut carried: HashSet<Uuid> = HashSet::new();
        if let Some(envelope) = draft.inflight
            && self.inflight.is_none()
        {
            match serde_json::from_value::<ProjectChanges>(envelope.input.clone()) {
                Ok(input) => {
                    let planned: Vec<Planned> = input
                        .features
                        .iter()
                        .filter_map(|c| planned_of(c, &envelope.expected_versions))
                        .collect();
                    carried.extend(planned.iter().map(Planned::id));
                    let meta = input.project.as_ref().map(|patch| self.meta_base.patched(patch));
                    self.own.insert(envelope.request_id.clone());
                    self.inflight = Some(Inflight {
                        envelope,
                        planned,
                        meta,
                    });
                    restored.resends = true;
                }
                Err(e) => restored.held.push(format!(
                    "gönderilmekte olan komut okunamadı ({e}); değişiklikleri taslaktan geri konuyor"
                )),
            }
        }
        // The metadata first: an object may sit on a layer it brings.
        let mut change = External::default();
        let mut meta_conflict = None;
        if let Some(m) = &draft.meta
            && self.can_edit_meta
        {
            let after = self.meta_base.patched(&m.patch);
            change.meta = Some(ExternalMeta {
                name: Some(after.name),
                settings: Some(after.settings),
                layers: Some(after.layers),
                styles: Some(after.styles),
            });
            if m.base != self.meta_version {
                meta_conflict = Some(self.meta_version.clone());
            }
        }
        let layers = match &change.meta {
            Some(ExternalMeta {
                layers: Some(layers),
                ..
            }) => layers.clone(),
            _ => doc.layers().nodes().to_vec(),
        };
        let mut conflicts = Vec::new();
        let mut touched = Vec::new();
        for (key, c) in draft.changes {
            let Ok(id) = Uuid::parse_str(&key) else {
                restored.held.push(format!(
                    "{key}: bir nesne kimliği değil; değişiklik atlandı"
                ));
                continue;
            };
            // Edited in this opening already: the newer edit wins.
            if self.plan(doc, id).is_some() {
                continue;
            }
            let server = self.known.get(&id);
            // Deleted here and on the server alike: nothing is left to do.
            if c.entity.is_none() && server.is_none() && doc.slot_of(id).is_none() {
                continue;
            }
            if let Some(e) = &c.entity
                && !is_layer(&layers, &e.base().layer_id)
            {
                restored.held.push(format!(
                    "{id}: “{}” katmanı çizimde yok; değişiklik bu cihazda saklanıyor, gönderilmedi",
                    e.base().layer_id
                ));
                self.held.insert(id, c);
                continue;
            }
            // The server moved past its base meanwhile. What the command on its way carries is
            // not checked here: its answer (or its refusal, a conflict then) settles it, and the
            // drawing shows it at once, since this device may have no connection to wait for.
            if !carried.contains(&id) && c.base.as_deref() != server.map(|t| t.version.as_str()) {
                conflicts.push(Conflict {
                    id: id.to_string(),
                    reason: if server.is_some() {
                        ConflictReason::Changed
                    } else {
                        ConflictReason::Deleted
                    },
                    server: server.map(|t| FeatureRecord {
                        id: id.to_string(),
                        version: t.version.clone(),
                        entity: t.entity.clone(),
                    }),
                    actual: server.map(|t| t.version.clone()),
                });
            }
            match c.entity {
                Some(entity) => change.put.push((id, entity)),
                None => {
                    if doc.slot_of(id).is_some() {
                        change.remove.push(id);
                    }
                }
            }
            touched.push(id);
        }
        let meta_back = change.meta.is_some();
        self.apply(doc, change)?;
        // It is this user's work, not sent yet.
        restored.changed = touched.len() + usize::from(meta_back);
        self.dirty.extend(touched);
        if meta_back {
            self.meta_dirty = true;
        }
        if let Some(actual) = meta_conflict {
            conflicts.push(Conflict {
                id: PROJECT_KEY.to_owned(),
                reason: ConflictReason::Project,
                server: None,
                actual: Some(actual),
            });
        }
        restored.conflicts = conflicts.len();
        for c in conflicts {
            self.conflict(c);
        }
        if restored.changed > 0 {
            doc.mark_unsaved();
        }
        if self.conflicts.is_empty() && self.can_write && !self.state.ended() {
            self.state = if self.pending() > 0 || self.inflight.is_some() {
                SaveState::Pending
            } else {
                SaveState::Saved
            };
        }
        Ok(restored)
    }
}
