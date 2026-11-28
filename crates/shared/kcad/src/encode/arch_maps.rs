//! The architectural objects whose fields are written as one map (a beam, a roof, a railing, a mass): each the
//! drawing's only, as it stands on the drawing's storeys, and written only as its contract allows it.

use kentos_contracts::EntityId;

use super::Encoder;
use super::objects::Val;
use crate::cbor::Seg;
use crate::error::{Code, KcadError};

impl<'d> Encoder<'d> {
    /// Pushes `fields` under `name`: refused in a block's definition (`uid` none) and when its contract finds
    /// `problem` with it; `what` names it in the refusal ("kiriş").
    pub(super) fn arch_map<T: serde::Serialize>(
        &mut self,
        uid: Option<&'d EntityId>,
        kind: &'static str,
        what: &str,
        problem: Option<(String, String)>,
        (name, fields): (&'static str, &T),
        f: &mut Vec<(&'static str, Val<'d>)>,
    ) -> Result<(), KcadError> {
        if uid.is_none() {
            self.path.push(Seg::Name(kind));
            return Err(self.fail(Code::BadValue, &format!("blok tanımında {what} olamaz")));
        }
        if let Some((_, problem)) = problem {
            self.path.push(Seg::Name(kind));
            return Err(self.fail(Code::BadValue, &problem));
        }
        f.push((name, Val::Json(serde_json::to_value(fields).unwrap_or_default())));
        Ok(())
    }
}
