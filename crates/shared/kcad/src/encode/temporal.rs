//! A layer's time setting, a group's scenario and a scenario layer's base
//! layer (document schema 34, docs/specs/kcad-v2.md §6.5, docs/adr/0210 §2):
//! each checked by the contract's rules, as the readers check it, its keys
//! sorted as RFC 8949 sorts them (`end`, `key`, `start`, `cumulative`). An
//! optional field is written only when it has a value, Birikimli only when true.

use kentos_contracts::{LayerNodeType, LayerTime, ScenarioInfo};

use super::Encoder;
use crate::cbor::Seg;
use crate::error::{Code, KcadError};

impl<'d> Encoder<'d> {
    /// A layer's time setting; a group has none.
    pub(super) fn layer_time(
        &mut self,
        kind: LayerNodeType,
        time: &'d LayerTime,
    ) -> Result<(), KcadError> {
        if kind == LayerNodeType::Group {
            return Err(self.fail(
                Code::BadValue,
                "grubun zamanı olmaz; zaman yalnız katmanındır",
            ));
        }
        if let Some(problem) = time.problem() {
            return Err(self.fail(Code::BadValue, &problem));
        }
        let n = 1
            + usize::from(time.end.is_some())
            + usize::from(time.key.is_some())
            + usize::from(time.cumulative);
        self.open(n, true)?;
        if let Some(end) = &time.end {
            self.key("end");
            self.at(Seg::Name("end"), |e| e.text(end))?;
        }
        if let Some(key) = &time.key {
            self.key("key");
            self.at(Seg::Name("key"), |e| e.text(key))?;
        }
        self.key("start");
        self.at(Seg::Name("start"), |e| e.text(&time.start))?;
        if time.cumulative {
            self.key("cumulative");
            self.w.bool(true);
        }
        self.close();
        Ok(())
    }

    /// A group's scenario; a layer has none.
    pub(super) fn scenario(
        &mut self,
        kind: LayerNodeType,
        scenario: &'d ScenarioInfo,
    ) -> Result<(), KcadError> {
        if kind == LayerNodeType::Layer {
            return Err(self.fail(
                Code::BadValue,
                "katman senaryo olmaz; yalnız grup senaryodur",
            ));
        }
        if let Some(problem) = scenario.problem() {
            return Err(self.fail(Code::BadValue, &problem));
        }
        self.open(usize::from(scenario.note.is_some()), true)?;
        if let Some(note) = &scenario.note {
            self.key("note");
            self.at(Seg::Name("note"), |e| e.text(note))?;
        }
        self.close();
        Ok(())
    }

    /// A scenario layer's base layer; a group stands for none.
    pub(super) fn replaces(&mut self, kind: LayerNodeType, base: &'d str) -> Result<(), KcadError> {
        if kind == LayerNodeType::Group {
            return Err(self.fail(
                Code::BadValue,
                "grup bir katmanın yerine geçmez; yalnız katman geçer",
            ));
        }
        self.text(base)
    }
}
