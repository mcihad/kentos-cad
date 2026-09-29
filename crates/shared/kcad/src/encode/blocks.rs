//! Block definitions written in document schema 6 (docs/specs/kcad-v2.md
//! §6.9, docs/adr/0144): each its id, base point, name, objects (written as
//! the drawing's are, without persistent ids: they are local to the
//! definition), attribute definitions and description; keys in encoded order.

use kentos_contracts::{AttributeDefinition, BlockDefinition};

use super::Encoder;
use crate::cbor::Seg;
use crate::error::KcadError;

impl<'d> Encoder<'d> {
    pub(super) fn blocks(&mut self, list: &'d [BlockDefinition]) -> Result<(), KcadError> {
        self.open(list.len(), false)?;
        let mut fields = Vec::with_capacity(16);
        for (i, block) in list.iter().enumerate() {
            self.path.push(Seg::Index(i));
            let n = 4
                + usize::from(!block.attributes.is_empty())
                + usize::from(block.description.is_some());
            self.open(n, true)?;
            // id (2), base name (4), entities (8), attributes (10), description (11).
            self.key("id");
            self.at(Seg::Name("id"), |e| e.id(&block.id.0))?;
            self.key("base");
            self.at(Seg::Name("base"), |e| e.point(&block.base))?;
            self.key("name");
            self.at(Seg::Name("name"), |e| e.text(&block.name))?;
            self.key("entities");
            self.path.push(Seg::Name("entities"));
            self.open(block.entities.len(), false)?;
            for (j, entity) in block.entities.iter().enumerate() {
                self.path.push(Seg::Index(j));
                self.object(entity, None, &mut fields)?;
                self.path.pop();
            }
            self.close();
            self.path.pop();
            if !block.attributes.is_empty() {
                self.key("attributes");
                self.at(Seg::Name("attributes"), |e| {
                    e.attribute_definitions(&block.attributes)
                })?;
            }
            if let Some(text) = &block.description {
                self.key("description");
                self.at(Seg::Name("description"), |e| e.text(text))?;
            }
            self.close();
            self.path.pop();
        }
        self.close();
        Ok(())
    }

    fn attribute_definitions(&mut self, list: &'d [AttributeDefinition]) -> Result<(), KcadError> {
        self.open(list.len(), false)?;
        for (i, a) in list.iter().enumerate() {
            self.path.push(Seg::Index(i));
            let n = 4 + usize::from(a.value.is_some()) + usize::from(a.prompt.is_some());
            self.open(n, true)?;
            // p (1), tag (3), value (5), height prompt (6), rotation (8).
            self.key("p");
            self.at(Seg::Name("p"), |e| e.point(&a.p))?;
            self.key("tag");
            self.at(Seg::Name("tag"), |e| e.text(&a.tag))?;
            if let Some(value) = &a.value {
                self.key("value");
                self.at(Seg::Name("value"), |e| e.text(value))?;
            }
            self.key("height");
            self.at(Seg::Name("height"), |e| e.float(a.height))?;
            if let Some(prompt) = &a.prompt {
                self.key("prompt");
                self.at(Seg::Name("prompt"), |e| e.text(prompt))?;
            }
            self.key("rotation");
            self.at(Seg::Name("rotation"), |e| e.float(a.rotation))?;
            self.close();
            self.path.pop();
        }
        self.close();
        Ok(())
    }
}
