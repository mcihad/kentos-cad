//! What a product command runs against (TODOS.md CMD-06).

use kentos_domain::Document;
use kentos_geometry_core::store::Store;

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
}

impl<'a> ExecutionContext<'a> {
    pub fn new(doc: &'a mut Document) -> Self {
        Self { doc, store: None }
    }

    /// The context with the host's store of `doc`, which must hold the document as it is now.
    pub fn with_store(doc: &'a mut Document, store: &'a Store) -> Self {
        Self {
            doc,
            store: Some(store),
        }
    }
}
