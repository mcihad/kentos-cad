//! Following an open database project (docs/adr/0040): its committed events
//! after the sync's cursor (`GET …/events`, the route the web replays from),
//! and what the server has now of the objects they name.
//!
//! The desktop asks every few seconds while a project is open, and again at
//! once while a page comes full. The web's live channel (the WebSocket) is a
//! later step for the desktop: it would only bring the same events sooner. A
//! cursor the server cannot continue from (older than the events it still
//! keeps) answers `resync_required`: the project is opened again.

use std::future::Future;

use kentos_contracts::{EventPage, FeatureRecord};
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

/// What the server has now of what `incoming` names: the objects others
/// created or changed (the ones it still has) and, when the metadata
/// changed, the project's info. Passing failures are tried again.
pub fn fetch(
    cloud: &Cloud,
    tenant: Uuid,
    project: Uuid,
    incoming: &Incoming,
) -> impl Future<Output = Result<Remote, ApiFailure>> + Send + 'static {
    let cloud = cloud.clone();
    let ids = incoming.fetch.clone();
    let meta = incoming.meta;
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
        Ok(Remote { records, info })
    })
}
