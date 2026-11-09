//! The parcel register's undo step (plan/map-cad.md MAP-08, docs/adr/4220). The register lives in the project's
//! settings, which are otherwise outside the model's undo; a document written to it is a step of its own.

use std::sync::Arc;

use kentos_contracts::ParcelRegister;

use crate::{Document, history::Op};

impl Document {
    /// The drawing's parcel register; none in a drawing without one.
    pub fn parcel_register(&self) -> Option<&ParcelRegister> {
        self.settings.parcel_register.as_ref()
    }

    /// Replaces the register as one undo step named `label` (inside an open transaction or group, part of it); an
    /// empty one is none. Nothing happens when it is the same.
    pub fn set_parcel_register(&mut self, next: Option<ParcelRegister>, label: &str) {
        let next = next.filter(|r| !r.is_empty());
        if self.settings.parcel_register == next {
            return;
        }
        let before = Arc::new(std::mem::replace(&mut self.settings.parcel_register, next));
        let after = Arc::new(self.settings.parcel_register.clone());
        self.record(vec![Op::ParcelRegister { before, after }], label);
    }
}
