//! The template sync plan (design §13): what to download, upload, delete or
//! keep, from the device's copies and the cloud's list. Pure and
//! deterministic (actions in id order); the network is the platform's. No
//! path loses data silently: a conflict keeps both, a change the other side
//! deleted stays here and is said.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// A template on this device, as the sync sees it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct LocalTemplate {
    /// The cloud's id (the device's copy keeps it).
    pub id: String,
    pub name: String,
    /// The cloud revision this copy came from; none: never uploaded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub base_revision: Option<u32>,
    /// Changed here since.
    #[serde(default)]
    pub dirty: bool,
    /// Deleted here (a tombstone kept until the cloud hears of it).
    #[serde(default)]
    pub deleted: bool,
}

/// A template in the cloud's list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct RemoteTemplate {
    pub id: String,
    pub name: String,
    pub revision: u32,
    #[serde(default)]
    pub deleted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SyncTarget {
    pub id: String,
    pub revision: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SyncId {
    pub id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SyncUpload {
    pub id: String,
    /// `sheet.template.update`'s `expectedRevision`.
    pub expected_revision: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SyncCopy {
    pub id: String,
    /// The new template's name: “… (bu cihazdaki kopya)”.
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SyncNotice {
    pub id: String,
    /// Turkish, for the user.
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum SyncAction {
    /// Fetch this revision and keep it as the device's copy.
    Download(SyncTarget),
    /// Upload the device's copy as a new revision.
    Upload(SyncUpload),
    /// Upload a template never uploaded (`sheet.template.create`).
    Create(SyncId),
    /// A conflict: the device's copy goes up as a new template under this name (the cloud's is downloaded too).
    UploadCopy(SyncCopy),
    /// Remove the device's copy.
    RemoveLocal(SyncId),
    /// Keep the device's copy as a template of this device only (its cloud one is gone), and say so.
    KeepLocal(SyncNotice),
    /// Delete it in the cloud (`sheet.template.delete` with this revision).
    DeleteRemote(SyncTarget),
    /// The cloud's stays (it changed after the device deleted it), and is downloaded back; said to the user.
    KeepRemote(SyncNotice),
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SyncPlan {
    pub actions: Vec<SyncAction>,
}

pub const COPY_SUFFIX: &str = " (bu cihazdaki kopya)";

/// The plan for the device's templates and the cloud's list (design §13's table).
pub fn plan_sync(local: &[LocalTemplate], remote: &[RemoteTemplate]) -> SyncPlan {
    let mut ids: BTreeMap<&str, (Option<&LocalTemplate>, Option<&RemoteTemplate>)> =
        BTreeMap::new();
    for l in local {
        ids.entry(l.id.as_str()).or_default().0 = Some(l);
    }
    for r in remote {
        ids.entry(r.id.as_str()).or_default().1 = Some(r);
    }
    let mut actions = Vec::new();
    for (id, pair) in ids {
        let id = id.to_owned();
        match pair {
            (None, None) => {}
            // Only in the cloud: a new template, or a tombstone of one this device never had.
            (None, Some(r)) => {
                if !r.deleted {
                    actions.push(SyncAction::Download(SyncTarget { id, revision: r.revision }));
                }
            }
            // Only here.
            (Some(l), None) => match (l.base_revision, l.deleted) {
                (None, false) => actions.push(SyncAction::Create(SyncId { id })),
                (None, true) => actions.push(SyncAction::RemoveLocal(SyncId { id })),
                // It was in the cloud and is gone from its list: as if deleted there.
                (Some(_), true) => actions.push(SyncAction::RemoveLocal(SyncId { id })),
                (Some(_), false) if l.dirty => actions.push(SyncAction::KeepLocal(SyncNotice {
                    message: format!("“{}” bulutta silinmiş; bu cihazdaki değişiklikleriniz “bu cihazda” şablon olarak kaldı.", l.name),
                    id,
                })),
                (Some(_), false) => actions.push(SyncAction::RemoveLocal(SyncId { id })),
            },
            (Some(l), Some(r)) => {
                let base = l.base_revision.unwrap_or(0);
                if r.deleted {
                    if l.dirty && !l.deleted {
                        actions.push(SyncAction::KeepLocal(SyncNotice {
                            message: format!(
                                "“{}” bulutta silinmiş; bu cihazdaki değişiklikleriniz “bu cihazda” şablon olarak kaldı.",
                                l.name
                            ),
                            id,
                        }));
                    } else {
                        actions.push(SyncAction::RemoveLocal(SyncId { id }));
                    }
                } else if l.deleted {
                    if r.revision == base {
                        actions.push(SyncAction::DeleteRemote(SyncTarget { id, revision: r.revision }));
                    } else {
                        actions.push(SyncAction::KeepRemote(SyncNotice {
                            id: id.clone(),
                            message: format!(
                                "“{}” siz sildikten sonra başka bir yerde değişti; bulutta kaldı ve yeniden indirildi.",
                                r.name
                            ),
                        }));
                        actions.push(SyncAction::Download(SyncTarget { id, revision: r.revision }));
                    }
                } else if l.dirty {
                    if r.revision == base && l.base_revision.is_some() {
                        actions.push(SyncAction::Upload(SyncUpload {
                            id,
                            expected_revision: base,
                        }));
                    } else {
                        actions.push(SyncAction::UploadCopy(SyncCopy {
                            id: id.clone(),
                            name: format!("{}{COPY_SUFFIX}", l.name),
                        }));
                        actions.push(SyncAction::Download(SyncTarget { id, revision: r.revision }));
                    }
                } else if r.revision != base {
                    actions.push(SyncAction::Download(SyncTarget { id, revision: r.revision }));
                }
            }
        }
    }
    SyncPlan { actions }
}
