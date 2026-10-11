//! Schema 40 (docs/specs/kcad-v2.md §6.5, §6.6, docs/adr/0225): an object's
//! `attachments`, a layer's `actions` and `card`, checked by the contract's
//! rules as the readers check them, their keys sorted as RFC 8949 sorts
//! them. A field is written only when it has a value; a card's switches
//! only when off.

use kentos_contracts::{
    Attachment, InfoCard, LayerAction, LayerNodeType, actions_problem, attachments_problem,
};

use super::Encoder;
use crate::cbor::Seg;
use crate::error::{Code, KcadError};

impl<'d> Encoder<'d> {
    /// An object's attachments, in their order.
    pub(super) fn attachments(&mut self, list: &'d [Attachment]) -> Result<(), KcadError> {
        if let Some(problem) = attachments_problem(list) {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.open(list.len(), false)?;
        for (i, a) in list.iter().enumerate() {
            self.at(Seg::Index(i), |e| {
                let n = 1 + [&a.url, &a.file, &a.note, &a.asset]
                    .iter()
                    .filter(|v| v.is_some())
                    .count();
                e.open(n, true)?;
                // url (3), file name note (4), asset (5).
                if let Some(url) = &a.url {
                    e.key("url");
                    e.at(Seg::Name("url"), |e| e.text(url))?;
                }
                if let Some(file) = &a.file {
                    e.key("file");
                    e.at(Seg::Name("file"), |e| e.text(file))?;
                }
                e.key("name");
                e.at(Seg::Name("name"), |e| e.text(&a.name))?;
                if let Some(note) = &a.note {
                    e.key("note");
                    e.at(Seg::Name("note"), |e| e.text(note))?;
                }
                if let Some(asset) = &a.asset {
                    e.key("asset");
                    e.at(Seg::Name("asset"), |e| e.text(asset))?;
                }
                e.close();
                Ok(())
            })?;
        }
        self.close();
        Ok(())
    }

    /// A layer's actions, in their order; a group has none.
    pub(super) fn layer_actions(
        &mut self,
        kind: LayerNodeType,
        actions: &'d [LayerAction],
    ) -> Result<(), KcadError> {
        if kind == LayerNodeType::Group {
            return Err(self.fail(
                Code::BadValue,
                "grubun eylemi olmaz; eylemler yalnız katmanındır",
            ));
        }
        if let Some(problem) = actions_problem(actions) {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.open(actions.len(), false)?;
        for (i, a) in actions.iter().enumerate() {
            self.at(Seg::Index(i), |e| {
                e.open(2, true)?;
                // name (4), target (6).
                e.key("name");
                e.at(Seg::Name("name"), |e| e.text(&a.name))?;
                e.key("target");
                e.at(Seg::Name("target"), |e| e.text(&a.target))?;
                e.close();
                Ok(())
            })?;
        }
        self.close();
        Ok(())
    }

    /// A layer's information card; a group has none.
    pub(super) fn info_card(
        &mut self,
        kind: LayerNodeType,
        card: &'d InfoCard,
    ) -> Result<(), KcadError> {
        if kind == LayerNodeType::Group {
            return Err(self.fail(
                Code::BadValue,
                "grubun bilgi kartı olmaz; kart yalnız katmanındır",
            ));
        }
        if let Some(problem) = card.problem() {
            return Err(self.fail(Code::BadValue, &problem));
        }
        let fields = !card.fields.is_empty();
        self.open(
            usize::from(card.text.is_some())
                + usize::from(card.title.is_some())
                + usize::from(fields)
                + usize::from(!card.actions)
                + usize::from(!card.attachments),
            true,
        )?;
        // text (4), title (5), fields (6), actions (7), attachments (11).
        if let Some(text) = &card.text {
            self.key("text");
            self.at(Seg::Name("text"), |e| e.text(text))?;
        }
        if let Some(title) = &card.title {
            self.key("title");
            self.at(Seg::Name("title"), |e| e.text(title))?;
        }
        if fields {
            self.key("fields");
            self.at(Seg::Name("fields"), |e| {
                e.open(card.fields.len(), false)?;
                for (i, f) in card.fields.iter().enumerate() {
                    e.at(Seg::Index(i), |e| e.text(f))?;
                }
                e.close();
                Ok(())
            })?;
        }
        if !card.actions {
            self.key("actions");
            self.w.bool(false);
        }
        if !card.attachments {
            self.key("attachments");
            self.w.bool(false);
        }
        self.close();
        Ok(())
    }
}
