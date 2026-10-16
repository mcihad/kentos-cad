//! Block definitions written in document schema 6, or 7 when an attribute
//! definition has an alignment or a width factor (docs/specs/kcad-v2.md
//! §6.9, docs/adr/0144, docs/adr/0145): each its id, base point, name,
//! objects (written as the drawing's are, without persistent ids: they are
//! local to the definition), attribute definitions and description; keys in
//! encoded order.

use kentos_contracts::{AttributeDefinition, BlockDefinition, width_factor_ok};

use super::Encoder;
use super::objects::width_factor_words;
use crate::cbor::Seg;
use crate::error::{Code, KcadError};

/// A block's unit as a file writes it (docs/adr/0315 §4).
pub(crate) fn units_name(u: kentos_contracts::BlockUnits) -> &'static str {
    use kentos_contracts::BlockUnits as B;
    match u {
        B::Mm => "mm",
        B::Cm => "cm",
        B::M => "m",
        B::In => "in",
        B::Ft => "ft",
        B::Unitless => "unitless",
    }
}

impl<'d> Encoder<'d> {
    pub(super) fn blocks(&mut self, list: &'d [BlockDefinition]) -> Result<(), KcadError> {
        self.open(list.len(), false)?;
        let mut fields = Vec::with_capacity(16);
        for (i, block) in list.iter().enumerate() {
            self.path.push(Seg::Index(i));
            let n = 4
                + usize::from(!block.attributes.is_empty())
                + usize::from(block.description.is_some())
                + usize::from(block.units.is_some())
                + usize::from(block.card.is_some())
                + usize::from(block.dynamic.is_some());
            self.open(n, true)?;
            // id (2), base card name (4), units (5), entities (8), attributes (10), description (11).
            self.key("id");
            self.at(Seg::Name("id"), |e| e.id(&block.id.0))?;
            self.key("base");
            self.at(Seg::Name("base"), |e| e.point(&block.base))?;
            // Its library card (docs/adr/0317 §4).
            if let Some(card) = &block.card {
                self.key("card");
                self.at(Seg::Name("card"), |e| e.card(card))?;
            }
            self.key("name");
            self.at(Seg::Name("name"), |e| e.text(&block.name))?;
            // The unit it is drawn in (docs/adr/0315 §4).
            if let Some(u) = block.units {
                self.key("units");
                self.at(Seg::Name("units"), |e| {
                    e.w.text(units_name(u));
                    Ok(())
                })?;
            }
            // Its parameters and actions (docs/adr/0318), as their JSON.
            if let Some(d) = &block.dynamic {
                let value = serde_json::to_value(d)
                    .map_err(|e| self.fail(Code::BadValue, &format!("dinamik bölüm yazılamadı: {e}")))?;
                self.key("dynamic");
                self.path.push(Seg::Name("dynamic"));
                self.json(&value)?;
                self.path.pop();
            }
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

    /// A block's card: tags (4), version (7), category (8); a version of 0, an empty category or an empty, folded
    /// twice or untrimmed tag is refused, as the reader refuses it.
    fn card(&mut self, card: &'d kentos_contracts::BlockCard) -> Result<(), KcadError> {
        if let Some(problem) = kentos_contracts::blocks::card_problem(card) {
            return Err(self.fail(Code::BadValue, &problem));
        }
        let n = 1 + usize::from(!card.tags.is_empty()) + usize::from(card.category.is_some());
        self.open(n, true)?;
        if !card.tags.is_empty() {
            self.key("tags");
            self.at(Seg::Name("tags"), |e| {
                e.open(card.tags.len(), false)?;
                for t in &card.tags {
                    e.text(t)?;
                }
                e.close();
                Ok(())
            })?;
        }
        self.key("version");
        self.at(Seg::Name("version"), |e| {
            e.w.uint(u64::from(card.version));
            Ok(())
        })?;
        if let Some(c) = &card.category {
            self.key("category");
            self.at(Seg::Name("category"), |e| e.text(c))?;
        }
        self.close();
        Ok(())
    }

    fn attribute_definitions(&mut self, list: &'d [AttributeDefinition]) -> Result<(), KcadError> {
        self.open(list.len(), false)?;
        for (i, a) in list.iter().enumerate() {
            self.path.push(Seg::Index(i));
            let o = &a.options;
            let n = 4
                + usize::from(a.value.is_some())
                + usize::from(a.prompt.is_some())
                + usize::from(a.align.is_some())
                + usize::from(a.width_factor.is_some())
                + [o.invisible, o.constant, o.verify, o.preset, o.movable, o.multiline]
                    .iter()
                    .filter(|b| **b)
                    .count()
                + usize::from(o.box_width.is_some())
                + usize::from(o.field.is_some());
            if let Some(problem) = kentos_contracts::blocks::options_problem(o) {
                self.path.push(Seg::Name("options"));
                return Err(self.fail(Code::BadValue, &problem));
            }
            if let Some(w) = a.width_factor
                && w.is_finite()
                && !width_factor_ok(w)
            {
                self.path.push(Seg::Name("widthFactor"));
                return Err(self.fail(Code::BadValue, &width_factor_words(w)));
            }
            self.open(n, true)?;
            // p (1), tag (3), align field value (5), height preset prompt verify (6), movable (7), boxWidth
            // constant rotation (8), invisible multiline (9), widthFactor (11). An option only when on (docs/adr/0317 §1).
            let flag = |e: &mut Self, name: &'static str, on: bool| -> Result<(), KcadError> {
                if on {
                    e.key(name);
                    e.at(Seg::Name(name), |e| {
                        e.w.bool(true);
                        Ok(())
                    })?;
                }
                Ok(())
            };
            self.key("p");
            self.at(Seg::Name("p"), |e| e.point(&a.p))?;
            self.key("tag");
            self.at(Seg::Name("tag"), |e| e.text(&a.tag))?;
            if let Some(align) = a.align {
                self.key("align");
                self.at(Seg::Name("align"), |e| e.text(align.name()))?;
            }
            if let Some(field) = &o.field {
                self.key("field");
                self.at(Seg::Name("field"), |e| e.text(field))?;
            }
            if let Some(value) = &a.value {
                self.key("value");
                self.at(Seg::Name("value"), |e| e.text(value))?;
            }
            self.key("height");
            self.at(Seg::Name("height"), |e| e.float(a.height))?;
            flag(self, "preset", o.preset)?;
            if let Some(prompt) = &a.prompt {
                self.key("prompt");
                self.at(Seg::Name("prompt"), |e| e.text(prompt))?;
            }
            flag(self, "verify", o.verify)?;
            flag(self, "movable", o.movable)?;
            if let Some(w) = o.box_width {
                self.key("boxWidth");
                self.at(Seg::Name("boxWidth"), |e| e.float(w))?;
            }
            flag(self, "constant", o.constant)?;
            self.key("rotation");
            self.at(Seg::Name("rotation"), |e| e.float(a.rotation))?;
            flag(self, "invisible", o.invisible)?;
            flag(self, "multiline", o.multiline)?;
            if let Some(w) = a.width_factor {
                self.key("widthFactor");
                self.at(Seg::Name("widthFactor"), |e| e.float(w))?;
            }
            self.close();
            self.path.pop();
        }
        self.close();
        Ok(())
    }
}
