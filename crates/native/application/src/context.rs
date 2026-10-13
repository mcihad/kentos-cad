//! What a product command runs against (TODOS.md CMD-06).

use kentos_domain::Document;
use kentos_geometry_core::store::Store;

use crate::call::CallState;
use crate::jobs::Progress;

/// A local command's world: the open document, and nothing else. There is no
/// tenant, project or actor, and nothing to authorize: the document is the
/// user's own. A cloud host will add the actor and the rights it takes from a
/// verified session, never from the input; the server's `CommandEnvelope`
/// stays its wire form (docs/adr/0022).
pub struct ExecutionContext<'a> {
    /// The only way a command changes anything; validate and plan only read it.
    pub doc: &'a mut Document,
    /// The host's geometry store of the document, kept in step with it (the
    /// desktop's), when it has one: the commands that ask which objects
    /// are shown where (Buda's and Uzat's boundaries, docs/adr/0250) read it
    /// instead of building their own. It changes no answer, only its cost.
    pub store: Option<&'a Store>,
    /// What the protocol asks of the write, when the command came through it
    /// (`call::call`, docs/adr/0253): the request, the plan it must equal;
    /// the write's summary comes back here.
    pub(crate) call: Option<CallState>,
    /// A job's progress and stop, when the command runs as one (jobs.rs,
    /// docs/adr/0253 §9).
    pub(crate) job: Option<&'a Progress>,
}

impl<'a> ExecutionContext<'a> {
    pub fn new(doc: &'a mut Document) -> Self {
        Self {
            doc,
            store: None,
            call: None,
            job: None,
        }
    }

    /// The context with the host's store of `doc`, which must hold the document as it is now.
    pub fn with_store(doc: &'a mut Document, store: &'a Store) -> Self {
        Self {
            doc,
            store: Some(store),
            call: None,
            job: None,
        }
    }

    /// The context with the host's store, when it has one.
    pub fn with_optional_store(doc: &'a mut Document, store: Option<&'a Store>) -> Self {
        Self {
            doc,
            store,
            call: None,
            job: None,
        }
    }

    /// Whether the job this command runs as was asked to stop (docs/adr/0253
    /// §9): a long loop asks between its parts and answers `cancelled`.
    pub fn stopped(&self) -> bool {
        self.job.is_some_and(Progress::stopped)
    }

    /// Says how far the job this command runs as has come: `done` of `total`.
    pub fn progress(&self, done: u64, total: u64) {
        if let Some(job) = self.job {
            job.set(done, total);
        }
    }
}
