//! The named views (document schema 52, docs/specs/kcad-v2.md §6.4.12,
//! docs/adr/0285 §1): checked by the contract's rule (`views_problem`), as
//! the readers check them; each view's keys sorted as RFC 8949 sorts them
//! (`name`, `twist`, `width`, `center`, `height`).

use kentos_contracts::{NamedView, views_problem};

use super::Encoder;
use crate::cbor::Seg;
use crate::error::{Code, KcadError};

impl<'d> Encoder<'d> {
    pub(super) fn named_views(&mut self, list: &'d [NamedView]) -> Result<(), KcadError> {
        if let Some(problem) = views_problem(list) {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.open(list.len(), false)?;
        for (i, v) in list.iter().enumerate() {
            self.at(Seg::Index(i), |e| {
                e.open(5 + usize::from(!v.id.is_empty()), true)?;
                // Schema 81 (docs/adr/0345 §2): a view's own id, first as RFC 8949 sorts the keys.
                if !v.id.is_empty() {
                    e.key("id");
                    e.at(Seg::Name("id"), |e| e.text(&v.id))?;
                }
                e.key("name");
                e.at(Seg::Name("name"), |e| e.text(&v.name))?;
                e.key("twist");
                e.at(Seg::Name("twist"), |e| e.float(v.twist))?;
                e.key("width");
                e.at(Seg::Name("width"), |e| e.float(v.width))?;
                e.key("center");
                e.at(Seg::Name("center"), |e| e.point(&v.center))?;
                e.key("height");
                e.at(Seg::Name("height"), |e| e.float(v.height))?;
                e.close();
                Ok(())
            })?;
        }
        self.close();
        Ok(())
    }
}
