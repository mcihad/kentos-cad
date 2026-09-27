//! A cloud project's history as the catalog shows it (the web's
//! app/cloud/history.ts; docs/adr/0034, 0038, 0087): a file project's
//! revisions and every project's named checkpoints, with who may do what
//! with them. The server decides every request; these rules only keep a
//! button from promising what it would refuse, and say which right is
//! missing.

use kentos_contracts::{
    Checkpoint, CheckpointKind, EventRecord, FileRevisions, PROJECT_CHECKPOINT_EVENT,
    PROJECT_FILE_COMMITTED, ProjectPermission, ProjectStorage,
};

/// What a checkpoint is, as a chip says it.
pub fn kind_label(kind: CheckpointKind) -> &'static str {
    match kind {
        CheckpointKind::Snapshot => "Anlık görüntü",
        CheckpointKind::Revision => "Adlandırılmış revizyon",
    }
}

/// What a checkpoint holds, in one line: a database project's data
/// revision, or a file project's revision.
pub fn point_text(c: &Checkpoint) -> String {
    match c.kind {
        CheckpointKind::Revision => format!("revizyon {}", c.revision),
        CheckpointKind::Snapshot => format!("veri revizyonu {}", c.revision),
    }
}

/// A project's history as the server answered it.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryData {
    pub storage: ProjectStorage,
    /// A file project's revisions, newest first; none for a database project.
    pub revisions: Option<FileRevisions>,
    /// The checkpoints, newest first; none when the account may not see the
    /// history (`project.history`).
    pub checkpoints: Option<Vec<Checkpoint>>,
}

/// Whether these events change what the history shows.
pub fn changes_history(events: &[EventRecord]) -> bool {
    events
        .iter()
        .any(|e| e.kind == PROJECT_CHECKPOINT_EVENT || e.kind == PROJECT_FILE_COMMITTED)
}

fn missing(right: ProjectPermission, what: &str) -> String {
    format!(
        "Bu projede {what} yetkiniz yok ({}); proje sahibine ya da yöneticisine başvurun.",
        right.name()
    )
}

/// Why the account may not name a checkpoint here (`feature.write`, not in
/// the archive, in a file project a saved revision), or none.
pub fn why_not_create(
    permissions: &[ProjectPermission],
    archived: bool,
    storage: ProjectStorage,
    revisions: Option<&FileRevisions>,
) -> Option<String> {
    if archived {
        return Some(
            "Arşivlenmiş projede kontrol noktası oluşturulmaz; önce arşivden çıkarın.".to_owned(),
        );
    }
    if !permissions.contains(&ProjectPermission::FeatureWrite) {
        return Some(missing(
            ProjectPermission::FeatureWrite,
            "kontrol noktası oluşturma",
        ));
    }
    if storage == ProjectStorage::File && revisions.is_none_or(|r| r.current.is_none()) {
        return Some(
            "Projenin henüz kaydedilmiş revizyonu yok; önce Kaydet ile bir revizyon yazın."
                .to_owned(),
        );
    }
    None
}

/// Why the account may not remove this checkpoint, or none: as the server
/// decides it (docs/adr/0034), one who may write the project, and then only
/// its maker or someone with `project.edit`; never in the archive.
pub fn why_not_delete(
    c: &Checkpoint,
    me: Option<&str>,
    permissions: &[ProjectPermission],
    archived: bool,
) -> Option<String> {
    if archived {
        return Some(
            "Arşivlenmiş projede kontrol noktası silinmez; önce arşivden çıkarın.".to_owned(),
        );
    }
    if !permissions.contains(&ProjectPermission::FeatureWrite) {
        return Some(missing(
            ProjectPermission::FeatureWrite,
            "kontrol noktası silme",
        ));
    }
    if Some(c.created_by.as_str()) == me || permissions.contains(&ProjectPermission::Edit) {
        return None;
    }
    Some(
        "Kontrol noktasını yalnız onu oluşturan ya da projeyi yöneten (project.edit) silebilir."
            .to_owned(),
    )
}

/// Why the account may not download a file project's revision
/// (`project.download`; listing them is `project.read`), or none.
pub fn why_not_download(permissions: &[ProjectPermission]) -> Option<String> {
    (!permissions.contains(&ProjectPermission::Download))
        .then(|| missing(ProjectPermission::Download, "indirme"))
}

/// Why the account may not download a checkpoint or restore a point of the
/// history (`project.history`, `project.download`), or none.
pub fn why_not_take(permissions: &[ProjectPermission], what: &str) -> Option<String> {
    if !permissions.contains(&ProjectPermission::History) {
        return Some(missing(ProjectPermission::History, what));
    }
    if !permissions.contains(&ProjectPermission::Download) {
        return Some(missing(ProjectPermission::Download, what));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use ProjectPermission as P;

    fn checkpoint(created_by: &str) -> Checkpoint {
        Checkpoint {
            id: "c1".into(),
            name: "Teslim".into(),
            note: None,
            kind: CheckpointKind::Snapshot,
            revision: "4".into(),
            size: "10".into(),
            sha256: "0".repeat(64),
            objects: None,
            created_by: created_by.into(),
            created_by_name: "X".into(),
            created_at: "2026-09-26T12:00:00Z".into(),
        }
    }

    // The web's roles (fakeServer.ts ROLE_PERMISSIONS), as the server gives them.
    const VIEWER: &[P] = &[P::Read, P::Download, P::History];
    const EDITOR: &[P] = &[
        P::Read,
        P::FeatureWrite,
        P::Comment,
        P::Download,
        P::History,
    ];
    const MANAGER: &[P] = &[
        P::Read,
        P::FeatureWrite,
        P::Edit,
        P::Comment,
        P::Download,
        P::History,
        P::Share,
    ];

    #[test]
    fn removing_a_checkpoint_is_its_makers_or_a_managers_never_in_the_archive() {
        assert_eq!(
            why_not_delete(&checkpoint("u1"), Some("u1"), EDITOR, false),
            None
        );
        assert!(
            why_not_delete(&checkpoint("u2"), Some("u1"), EDITOR, false).is_some_and(
                |w| w.contains("yalnız onu oluşturan ya da projeyi yöneten (project.edit)")
            )
        );
        assert_eq!(
            why_not_delete(&checkpoint("u2"), Some("u1"), MANAGER, false),
            None
        );
        // A viewer who made one while an editor may not remove it now: the server asks feature.write first.
        assert!(
            why_not_delete(&checkpoint("u1"), Some("u1"), VIEWER, false)
                .is_some_and(|w| w.contains("feature.write"))
        );
        assert!(
            why_not_delete(&checkpoint("u1"), Some("u1"), &P::ALL, true)
                .is_some_and(|w| w.contains("Arşivlenmiş"))
        );
    }

    #[test]
    fn naming_needs_write_and_a_saved_revision_taking_needs_history_and_download() {
        assert!(
            why_not_create(VIEWER, false, ProjectStorage::Database, None)
                .is_some_and(|w| w.contains("feature.write"))
        );
        let none = FileRevisions {
            current: None,
            revisions: Vec::new(),
        };
        assert!(
            why_not_create(EDITOR, false, ProjectStorage::File, Some(&none))
                .is_some_and(|w| w.contains("henüz kaydedilmiş revizyonu yok"))
        );
        assert_eq!(
            why_not_create(EDITOR, false, ProjectStorage::Database, None),
            None
        );
        assert!(
            why_not_take(&[P::Read, P::History], "indirme")
                .is_some_and(|w| w.contains("project.download"))
        );
        assert_eq!(why_not_take(VIEWER, "indirme"), None);
        // A revision is a file project's content: downloading it needs project.download, not the history.
        assert_eq!(why_not_download(&[P::Read, P::Download]), None);
        assert!(
            why_not_download(&[P::Read, P::History])
                .is_some_and(|w| w.contains("project.download"))
        );
    }
}
