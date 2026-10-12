//! Schema 41 (docs/specs/kcad-v2.md §6.5, docs/adr/0226 §3, §4): a layer's
//! `compose`, checked by the contract's rule as the readers check it, its
//! keys sorted as RFC 8949 sorts them. What says nothing (an opacity of 1,
//! the normal mode, a composition with nothing in it) is refused as the
//! readers refuse it: the documents keep a composition without them
//! (`LayerCompose::normalized`).

use kentos_contracts::{BlendMode, LayerCompose, LayerNodeType};

use super::Encoder;
use crate::cbor::Seg;
use crate::error::{Code, KcadError};

impl<'d> Encoder<'d> {
    /// A layer's composition; a group has none.
    pub(super) fn layer_compose(
        &mut self,
        kind: LayerNodeType,
        c: &LayerCompose,
    ) -> Result<(), KcadError> {
        if kind == LayerNodeType::Group {
            return Err(self.fail(
                Code::BadValue,
                "grubun katman çizimi olmaz; katman çizimi yalnız katmanındır",
            ));
        }
        if c.opacity == Some(1.0) {
            return Err(self.fail(
                Code::BadValue,
                "1 saydamlığı yazılmaz; katman opaksa alan boş bırakılır",
            ));
        }
        if c.blend == Some(BlendMode::Normal) {
            return Err(self.fail(
                Code::BadValue,
                "normal karışım kipi yazılmaz; kip normalse alan boş bırakılır",
            ));
        }
        if let Some(problem) = c.problem() {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.open(
            usize::from(c.mask.is_some())
                + usize::from(c.blend.is_some())
                + usize::from(c.opacity.is_some()),
            true,
        )?;
        // mask (4), blend (5), opacity (7).
        if let Some(mask) = &c.mask {
            self.key("mask");
            self.at(Seg::Name("mask"), |e| {
                e.open(
                    2 + usize::from(mask.labels) + usize::from(mask.markers),
                    true,
                )?;
                // size (4), labels (6), markers targets (7).
                e.key("size");
                e.at(Seg::Name("size"), |e| e.float(mask.size))?;
                if mask.labels {
                    e.key("labels");
                    e.w.bool(true);
                }
                if mask.markers {
                    e.key("markers");
                    e.w.bool(true);
                }
                e.key("targets");
                e.at(Seg::Name("targets"), |e| {
                    e.open(mask.targets.len(), false)?;
                    for (i, t) in mask.targets.iter().enumerate() {
                        e.at(Seg::Index(i), |e| e.text(t))?;
                    }
                    e.close();
                    Ok(())
                })?;
                e.close();
                Ok(())
            })?;
        }
        if let Some(blend) = c.blend {
            self.key("blend");
            self.w.text(blend.key());
        }
        if let Some(opacity) = c.opacity {
            self.key("opacity");
            self.at(Seg::Name("opacity"), |e| e.float(opacity))?;
        }
        self.close();
        Ok(())
    }
}
