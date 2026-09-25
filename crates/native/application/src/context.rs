//! What a product command runs against (TODOS.md CMD-06).

use kentos_domain::Document;

/// A local command's world: the open document, and nothing else. There is no
/// tenant, project or actor, and nothing to authorize: the document is the
/// user's own. A cloud host will add the actor and the rights it takes from a
/// verified session, never from the input; the server's `CommandEnvelope`
/// stays its wire form (docs/adr/0022).
pub struct ExecutionContext<'a> {
    /// The only way a command changes anything; validate and plan only read it.
    pub doc: &'a mut Document,
}

impl<'a> ExecutionContext<'a> {
    pub fn new(doc: &'a mut Document) -> Self {
        Self { doc }
    }
}
