//! Following an open database project (docs/adr/0040): its committed events
//! after the sync's cursor (`GET …/events`, the route the web replays from),
//! and what the server has now of the objects they name.
//!
//! The desktop asks again as soon as an answer comes: a request that finds
//! nothing new waits on the server for the project's next commit (a long
//! poll, [`wait`], docs/adr/0044), so another editor's change arrives within
//! a moment, over the same HTTP and TLS as every other request. A full page
//! means more wait. A cursor the server cannot continue from (older than the
//! events it still keeps, or beyond the newest) answers `resync_required`:
//! the project is opened again.

use std::future::Future;

use kentos_contracts::{BlockRecord, Entity, EventPage, FeatureRecord, FileRevisions, ProjectInfo};
use uuid::Uuid;

use crate::api::Cloud;
use crate::failure::ApiFailure;
use crate::runtime::run;
use crate::saving::retrying;
use crate::sync::{Incoming, Remote};

/// Objects asked for by id at once (the ids travel in the address).
pub const FETCH: usize = 500;

/// Events the server sends in one page at most: a full page means more wait.
pub const EVENTS_PAGE: usize = 500;

/// The committed events after `after`.
pub fn events(
    cloud: &Cloud,
    tenant: Uuid,
    project: Uuid,
    after: &str,
) -> impl Future<Output = Result<EventPage, ApiFailure>> + Send + 'static {
    cloud.events(tenant, project, after)
}

/// How long one request waits on the server for a commit (the server's most).
pub const WAIT: std::time::Duration = std::time::Duration::from_secs(25);

/// The committed events after `after`, waiting on the server up to [`WAIT`]
/// for the next commit when none is new: call it again as soon as it answers.
pub fn wait(
    cloud: &Cloud,
    tenant: Uuid,
    project: Uuid,
    after: &str,
) -> impl Future<Output = Result<EventPage, ApiFailure>> + Send + 'static {
    cloud.events_waiting(tenant, project, after, WAIT)
}

/// The project as the server has it now, passing failures tried again
/// first: a file project's resync asks it when the events it missed cannot
/// be replayed (docs/specs/file-revisions.md §2.4).
pub fn project_now(
    cloud: &Cloud,
    tenant: Uuid,
    project: Uuid,
) -> impl Future<Output = Result<ProjectInfo, ApiFailure>> + Send + 'static {
    let cloud = cloud.clone();
    run(async move { retrying(|| cloud.project(tenant, project)).await })
}

/// A file project's revisions as the server has them now, passing failures
/// tried again first: who saved the newest and when.
pub fn file_revisions_now(
    cloud: &Cloud,
    tenant: Uuid,
    project: Uuid,
) -> impl Future<Output = Result<FileRevisions, ApiFailure>> + Send + 'static {
    let cloud = cloud.clone();
    run(async move { retrying(|| cloud.file_revisions(tenant, project)).await })
}

/// The project's block definitions as the server has them now (`GET
/// …/blocks`, docs/adr/0144 §5), passing failures tried again first: a
/// definition's conflict is chosen on with them, and a refused removal of
/// one still placed puts it back from them.
pub fn blocks_now(
    cloud: &Cloud,
    tenant: Uuid,
    project: Uuid,
) -> impl Future<Output = Result<Vec<BlockRecord>, ApiFailure>> + Send + 'static {
    let cloud = cloud.clone();
    run(async move { Ok(retrying(|| cloud.blocks(tenant, project)).await?.blocks) })
}

/// What the server has now of what `incoming` names: the objects others
/// created or changed (the ones it still has), when the metadata changed
/// the project's info, and its block definitions when the events named some
/// or an object they bring is an insert (the drawing may lack its block).
/// Passing failures are tried again.
pub fn fetch(
    cloud: &Cloud,
    tenant: Uuid,
    project: Uuid,
    incoming: &Incoming,
) -> impl Future<Output = Result<Remote, ApiFailure>> + Send + 'static {
    let cloud = cloud.clone();
    let ids = incoming.fetch.clone();
    let meta = incoming.meta;
    let named = !incoming.blocks.is_empty();
    run(async move {
        let mut records: Vec<FeatureRecord> = Vec::with_capacity(ids.len());
        for part in ids.chunks(FETCH) {
            let page = retrying(|| cloud.features_by_id(tenant, project, part)).await?;
            records.extend(page.features);
        }
        let info = if meta {
            Some(retrying(|| cloud.project(tenant, project)).await?)
        } else {
            None
        };
        let blocks = if named
            || records
                .iter()
                .any(|r| matches!(r.entity, Entity::Insert(_)))
        {
            Some(retrying(|| cloud.blocks(tenant, project)).await?.blocks)
        } else {
            None
        };
        Ok(Remote {
            records,
            info,
            blocks,
        })
    })
}
