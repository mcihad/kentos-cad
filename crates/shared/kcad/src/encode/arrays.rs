//! The associative arrays (document schema 53, docs/specs/kcad-v2.md §6.14,
//! docs/adr/0300): each array's id, source block, first place, layout and
//! items, held to the contract's rules (`arrays_problem`) as the readers hold
//! them. Keys in the canonical order (length, then bytes).

use kentos_contracts::{
    ArrayFault, ArrayLayout, AssocArray, BlockDefinition, EntityId, arrays_problem,
};

use super::Encoder;
use crate::cbor::Seg;
use crate::error::{Code, KcadError};

impl<'d> Encoder<'d> {
    pub(super) fn arrays(
        &mut self,
        list: &'d [AssocArray],
        defined: &[BlockDefinition],
    ) -> Result<(), KcadError> {
        let blocks: Vec<_> = defined.iter().map(|b| b.id).collect();
        if let Some((at, why)) = arrays_problem(list, &blocks) {
            match at {
                ArrayFault::Array(i) => self.path.push(Seg::Index(i)),
                ArrayFault::Item(i, k) => {
                    self.path.push(Seg::Index(i));
                    self.path.push(Seg::Name("items"));
                    self.path.push(Seg::Index(k));
                }
            }
            return Err(self.fail(Code::BadValue, why));
        }
        self.open(list.len(), false)?;
        for (i, a) in list.iter().enumerate() {
            self.at(Seg::Index(i), |e| e.array(a))?;
        }
        self.close();
        Ok(())
    }

    fn array(&mut self, a: &'d AssocArray) -> Result<(), KcadError> {
        self.open(8 + usize::from(a.mirror), true)?;
        // p (1), id (2), block items scale (5), layout mirror (6), layerId (7), rotation (8).
        self.key("p");
        self.at(Seg::Name("p"), |e| e.point(&a.p))?;
        self.key("id");
        self.at(Seg::Name("id"), |e| e.id(&a.id.0))?;
        self.key("block");
        self.at(Seg::Name("block"), |e| e.id(&a.block.0))?;
        self.key("items");
        self.at(Seg::Name("items"), |e| {
            e.open(a.items.len(), false)?;
            for (k, item) in a.items.iter().enumerate() {
                e.at(Seg::Index(k), |e| {
                    e.open(
                        usize::from(item.uid.is_some()) + usize::from(item.kept),
                        true,
                    )?;
                    // uid (3), kept (4).
                    if let Some(uid) = &item.uid {
                        e.key("uid");
                        e.at(Seg::Name("uid"), |e| e.id(&uid.0))?;
                    }
                    if item.kept {
                        e.key("kept");
                        e.w.bool(true);
                    }
                    e.close();
                    Ok(())
                })?;
            }
            e.close();
            Ok(())
        })?;
        self.key("scale");
        self.at(Seg::Name("scale"), |e| e.float(a.scale))?;
        self.key("layout");
        self.at(Seg::Name("layout"), |e| e.array_layout(&a.layout))?;
        if a.mirror {
            self.key("mirror");
            self.w.bool(true);
        }
        self.key("layerId");
        self.at(Seg::Name("layerId"), |e| e.text(&a.layer_id))?;
        self.key("rotation");
        self.at(Seg::Name("rotation"), |e| e.float(a.rotation))?;
        self.close();
        Ok(())
    }

    fn array_layout(&mut self, layout: &'d ArrayLayout) -> Result<(), KcadError> {
        match layout {
            ArrayLayout::Grid { rows, cols, dx, dy } => {
                self.open(5, true)?;
                // dx dy (2), cols kind rows (4).
                self.key("dx");
                self.at(Seg::Name("dx"), |e| e.float(*dx))?;
                self.key("dy");
                self.at(Seg::Name("dy"), |e| e.float(*dy))?;
                self.key("cols");
                self.w.uint(u64::from(*cols));
                self.key("kind");
                self.w.text("grid");
                self.key("rows");
                self.w.uint(u64::from(*rows));
            }
            ArrayLayout::Polar {
                center,
                count,
                fill,
                rotate,
            } => {
                self.open(5, true)?;
                // fill kind (4), count (5), center rotate (6).
                self.key("fill");
                self.at(Seg::Name("fill"), |e| e.float(*fill))?;
                self.key("kind");
                self.w.text("polar");
                self.key("count");
                self.w.uint(u64::from(*count));
                self.key("center");
                self.at(Seg::Name("center"), |e| e.point(center))?;
                self.key("rotate");
                self.w.bool(*rotate);
            }
            ArrayLayout::Line { count, dx, dy, fit } => {
                self.open(4 + usize::from(*fit), true)?;
                // dx dy (2), fit (3), kind (4), count (5).
                self.key("dx");
                self.at(Seg::Name("dx"), |e| e.float(*dx))?;
                self.key("dy");
                self.at(Seg::Name("dy"), |e| e.float(*dy))?;
                if *fit {
                    self.key("fit");
                    self.w.bool(true);
                }
                self.key("kind");
                self.w.text("line");
                self.key("count");
                self.w.uint(u64::from(*count));
            }
            ArrayLayout::Path {
                path,
                count,
                spacing,
                align,
            } => {
                self.open(4 + usize::from(spacing.is_some()), true)?;
                // kind path (4), align count (5), spacing (7).
                self.key("kind");
                self.w.text("path");
                self.key("path");
                self.at(Seg::Name("path"), |e| {
                    let id = EntityId::parse(path)
                        .ok_or_else(|| e.fail(Code::BadValue, "yolun kimliği bir UUID olmalı"))?;
                    e.id(&id.0)
                })?;
                self.key("align");
                self.w.bool(*align);
                self.key("count");
                self.w.uint(u64::from(*count));
                if let Some(s) = spacing {
                    self.key("spacing");
                    self.at(Seg::Name("spacing"), |e| e.float(*s))?;
                }
            }
        }
        self.close();
        Ok(())
    }
}
