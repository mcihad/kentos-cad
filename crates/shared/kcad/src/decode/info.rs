//! Schema 40 (docs/specs/kcad-v2.md §6.5, §6.6, docs/adr/0225): an object's
//! `attachments`, a layer's `actions` and `card`, read field by field and
//! checked whole by the contract's rules (`attachments_problem`,
//! `actions_problem`, `InfoCard::problem`), as the writers check them. An
//! empty list and a card's switch written on are not written (canonical
//! bytes). The tree's own rule (`infos_problem`: not on a group, not on a
//! layer drawn from a service) is checked once the tree is read.

use kentos_contracts::{Attachment, InfoCard, LayerAction, actions_problem, attachments_problem};

use super::{list, map, text, unknown};
use crate::cbor::Reader;
use crate::error::{Code, KcadError};

pub(super) fn attachments(r: &mut Reader<'_>) -> Result<Vec<Attachment>, KcadError> {
    let at = r.position();
    let out = list(r, |r, _| {
        let mut a = Attachment::default();
        let mut name = None;
        map(r, |r, k| {
            match k {
                "url" => a.url = Some(text(r)?),
                "file" => a.file = Some(text(r)?),
                "name" => name = Some(text(r)?),
                "note" => a.note = Some(text(r)?),
                "asset" => a.asset = Some(text(r)?),
                _ => return Err(unknown(r)),
            }
            Ok(())
        })?;
        a.name = super::required(r, name, "name")?;
        Ok(a)
    })?;
    if out.is_empty() {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "nesnenin ek listesi boş; boş liste yazılmaz",
        ));
    }
    match attachments_problem(&out) {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(out),
    }
}

pub(super) fn layer_actions(r: &mut Reader<'_>) -> Result<Vec<LayerAction>, KcadError> {
    let at = r.position();
    let out = list(r, |r, _| {
        let (mut name, mut target) = (None, None);
        map(r, |r, k| {
            match k {
                "name" => name = Some(text(r)?),
                "target" => target = Some(text(r)?),
                _ => return Err(unknown(r)),
            }
            Ok(())
        })?;
        Ok(LayerAction {
            name: super::required(r, name, "name")?,
            target: super::required(r, target, "target")?,
        })
    })?;
    if out.is_empty() {
        return Err(r.fail_at(
            Code::BadValue,
            at,
            "katmanın eylem listesi boş; boş liste yazılmaz",
        ));
    }
    match actions_problem(&out) {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(out),
    }
}

pub(super) fn info_card(r: &mut Reader<'_>) -> Result<InfoCard, KcadError> {
    let at = r.position();
    let mut card = InfoCard::default();
    map(r, |r, k| {
        match k {
            "text" => card.text = Some(text(r)?),
            "title" => card.title = Some(text(r)?),
            "fields" => {
                let list_at = r.position();
                card.fields = list(r, |r, _| text(r))?;
                if card.fields.is_empty() {
                    return Err(r.fail_at(
                        Code::BadValue,
                        list_at,
                        "kartın alan listesi boş; boş liste yazılmaz",
                    ));
                }
            }
            "actions" | "attachments" => {
                let on = r.bool()?;
                if on {
                    return Err(r.fail(
                        Code::BadValue,
                        "kartın düğmesi açıkken yazılmaz; yalnız kapalısı (false) yazılır",
                    ));
                }
                if k == "actions" {
                    card.actions = false;
                } else {
                    card.attachments = false;
                }
            }
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    match card.problem() {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(card),
    }
}
