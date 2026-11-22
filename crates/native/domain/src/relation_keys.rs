//! A relation's codes (docs/adr/9135): what a child's field holds for its parent. A relation without a parent key ties
//! by the parent's persistent id (docs/adr/0405 §1); one with a key ties by the value of that parent field, an
//! institution's own code (a neighbourhood's `M1`), the spaces round it dropped. Every way of following a relation (its
//! children's deletion, a trace's related records, a form's related list, `cad.records.page`) asks here.

use kentos_contracts::{Entity, ProjectRelation};
use uuid::Uuid;

use crate::Document;

/// What a relation's children hold of their parent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParentKey {
    /// The parent's persistent id.
    Uid,
    /// The value of the parent's field of this name.
    Field(String),
    /// The key names a field the parent layer does not have: the relation ties nothing.
    Missing,
}

impl ParentKey {
    /// The code a parent `uid`, `e` gives its children; none for a parent whose key is empty (it has no child).
    pub fn code_of(&self, uid: Uuid, e: &Entity) -> Option<String> {
        match self {
            ParentKey::Uid => Some(uid.to_string()),
            ParentKey::Field(name) => e
                .base()
                .attrs
                .get(name)
                .map(|v| v.trim())
                .filter(|v| !v.is_empty())
                .map(str::to_owned),
            ParentKey::Missing => None,
        }
    }
}

impl Document {
    /// What `r`'s children hold of their parent: its persistent id, or its key field's value by the field's name.
    pub fn parent_key(&self, r: &ProjectRelation) -> ParentKey {
        let Some(key) = r.parent_key.as_deref() else {
            return ParentKey::Uid;
        };
        self.layers
            .get(&r.parent)
            .and_then(|n| n.fields.iter().find(|f| f.key() == key))
            .map_or(ParentKey::Missing, |f| ParentKey::Field(f.name.clone()))
    }
}
