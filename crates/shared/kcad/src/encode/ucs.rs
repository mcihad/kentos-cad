//! The user coordinate systems (document schema 49, docs/specs/kcad-v2.md
//! §6.4.11, docs/adr/0261 §1): checked by the contract's rule
//! (`ucs_problem`), as the readers check them; each map's keys sorted as
//! RFC 8949 sorts them (`name`, `angle`, `origin`).

use kentos_contracts::{NamedUcs, Ucs, ucs_problem};

use super::Encoder;
use crate::cbor::Seg;
use crate::error::{Code, KcadError};

impl<'d> Encoder<'d> {
    fn frame(&mut self, name: Option<&'d str>, u: &'d Ucs) -> Result<(), KcadError> {
        self.open(2 + usize::from(name.is_some()), true)?;
        if let Some(n) = name {
            self.key("name");
            self.at(Seg::Name("name"), |e| e.text(n))?;
        }
        self.key("angle");
        self.at(Seg::Name("angle"), |e| e.float(u.angle))?;
        self.key("origin");
        self.at(Seg::Name("origin"), |e| e.point(&u.origin))?;
        self.close();
        Ok(())
    }

    pub(super) fn ucs(&mut self, u: &'d Ucs) -> Result<(), KcadError> {
        if let Some(problem) = ucs_problem(Some(u), &[]) {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.frame(None, u)
    }

    pub(super) fn named_ucs(&mut self, list: &'d [NamedUcs]) -> Result<(), KcadError> {
        if let Some(problem) = ucs_problem(None, list) {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.open(list.len(), false)?;
        for (i, n) in list.iter().enumerate() {
            self.at(Seg::Index(i), |e| {
                // A named UCS's frame borrowed from the list, as written.
                e.open(3, true)?;
                e.key("name");
                e.at(Seg::Name("name"), |e| e.text(&n.name))?;
                e.key("angle");
                e.at(Seg::Name("angle"), |e| e.float(n.angle))?;
                e.key("origin");
                e.at(Seg::Name("origin"), |e| e.point(&n.origin))?;
                e.close();
                Ok(())
            })?;
        }
        self.close();
        Ok(())
    }
}
