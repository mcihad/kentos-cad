//! Schema 41 (docs/specs/kcad-v2.md §6.5, docs/adr/0226 §3, §4): a layer's
//! `compose`, read field by field and checked whole by the contract's rule
//! (`LayerCompose::problem`), as the writers check it. What says nothing is
//! not written (canonical bytes): an opacity of 1, the normal mode, a mask's
//! switch written off. The tree's own rule (`composes_problem`: not on a
//! group) is checked once the tree is read; a mask's targets the tree does not
//! have are kept (they are skipped when drawn, `mask_targets`).

use kentos_contracts::{BlendMode, LayerCompose, LayerMask};

use super::{list, map, text, unknown};
use crate::cbor::Reader;
use crate::error::{Code, KcadError};

pub(super) fn layer_compose(r: &mut Reader<'_>) -> Result<LayerCompose, KcadError> {
    let at = r.position();
    let mut c = LayerCompose::default();
    map(r, |r, k| {
        match k {
            "mask" => c.mask = Some(mask(r)?),
            "blend" => {
                let name = text(r)?;
                match BlendMode::from_key(&name) {
                    Some(BlendMode::Normal) => {
                        return Err(r.fail(
                            Code::BadValue,
                            "normal karışım kipi yazılmaz; kip normalse alan boş bırakılır",
                        ));
                    }
                    Some(mode) => c.blend = Some(mode),
                    None => {
                        return Err(
                            r.fail(Code::BadValue, &format!("bilinmeyen karışım kipi “{name}”"))
                        );
                    }
                }
            }
            "opacity" => {
                let o = r.float()?;
                if o == 1.0 {
                    return Err(r.fail(
                        Code::BadValue,
                        "1 saydamlığı yazılmaz; katman opaksa alan boş bırakılır",
                    ));
                }
                c.opacity = Some(o);
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    match c.problem() {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(c),
    }
}

fn mask(r: &mut Reader<'_>) -> Result<LayerMask, KcadError> {
    let mut m = LayerMask::default();
    let (mut size, mut targets) = (None, None);
    map(r, |r, k| {
        match k {
            "size" => size = Some(r.float()?),
            "labels" | "markers" => {
                if !r.bool()? {
                    return Err(r.fail(
                        Code::BadValue,
                        "maskenin kaynağı yalnız true yazılır; kapalıysa yazılmaz",
                    ));
                }
                if k == "labels" {
                    m.labels = true;
                } else {
                    m.markers = true;
                }
            }
            "targets" => targets = Some(list(r, |r, _| text(r))?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    m.size = super::required(r, size, "size")?;
    m.targets = super::required(r, targets, "targets")?;
    Ok(m)
}
