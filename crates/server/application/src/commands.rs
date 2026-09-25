//! The product commands a project's command route runs (docs/adr/0013): one
//! handler per catalog command marked `server` ([`crate::SERVER_COMMANDS`],
//! `tests/catalog.rs` keeps the two equal). Each checks its own version,
//! input and permissions against the caller's [`ProjectAccess`].

use kentos_contracts::{
    CommandEnvelope, CommitResult, PROJECT_ACCESS_REVOKE, PROJECT_CHANGES, PROJECT_SHARE,
    ProjectAccessChange,
};
use serde_json::Value;

use crate::access::ProjectAccess;
use crate::error::{AppError, AppResult};
use crate::{changes, sharing};

/// What a command answered.
#[derive(Clone, Debug, PartialEq)]
pub enum CommandOutcome {
    Changes(CommitResult),
    Access(ProjectAccessChange),
}

impl CommandOutcome {
    /// Something was committed now (not a retry's stored answer, not a no-op):
    /// the project's open connections are told.
    pub fn committed(&self) -> bool {
        match self {
            Self::Changes(r) => !r.replayed,
            Self::Access(r) => !r.replayed && r.changed,
        }
    }

    /// The answer as the API sends it (the command's own output type).
    pub fn to_json(&self) -> Value {
        match self {
            Self::Changes(r) => serde_json::to_value(r),
            Self::Access(r) => serde_json::to_value(r),
        }
        .expect("command results serialize")
    }
}

/// Runs the envelope's command on the project `access` names.
pub async fn run(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
    envelope: CommandEnvelope,
) -> AppResult<CommandOutcome> {
    match envelope.command_name.as_str() {
        PROJECT_CHANGES => changes::commit(db, access, envelope)
            .await
            .map(CommandOutcome::Changes),
        PROJECT_SHARE => sharing::share(db, access, envelope)
            .await
            .map(CommandOutcome::Access),
        PROJECT_ACCESS_REVOKE => sharing::revoke(db, access, envelope)
            .await
            .map(CommandOutcome::Access),
        other => Err(AppError::invalid(format!("Bilinmeyen komut: {other}"))),
    }
}
