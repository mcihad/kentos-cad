//! Eklenti denetimi's rule picker (docs/adr/8057 §93): every audit rule of an active extension that joins this
//! program's audit (`entrypoints.audits[].domain`), whether its extension puts a button on the ribbon or not. A utility
//! extension opens no tab: without this the national regulation's rules (Türkiye mevzuatı) ran only from an agent or
//! a cloud job, and the user could not run them on the drawing in front.

use iced::widget::row;
use iced::{Center, Element};
use kentos_extension_host::ContributionKind;
use kentos_ui::label;
use kentos_ui::widget::{Choice, Select};

use super::{Event, msg};
use crate::app::{App, Message};
use crate::exchange::words;

/// A rule the picker offers: its extension, its local id and its words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuleChoice {
    pub(crate) ext: String,
    pub(crate) local: String,
    pub(crate) words: String,
}

impl App {
    /// Whether an extension offers the active module an audit rule, as [`App::ext_audit_choices`] would list one:
    /// Eklenti denetimi shows only then (docs/adr/9195 §5). The bottom panel asks it every frame, so it lists nothing.
    pub(crate) fn offers_ext_audits(&self) -> bool {
        let (Some(registry), Some(module)) = (self.extensions.registry.as_ref(), self.active_module()) else {
            return false;
        };
        registry
            .contributions()
            .iter()
            .filter(|c| c.kind == ContributionKind::Audit)
            .any(|c| {
                let ext = c.lease.extension.as_str();
                let local = c
                    .id
                    .strip_prefix("ext.")
                    .and_then(|rest| rest.strip_prefix(ext))
                    .and_then(|rest| rest.strip_prefix('.'));
                local.is_some_and(|local| {
                    registry.manifest(ext).is_some_and(|m| {
                        m.entrypoints
                            .audits
                            .iter()
                            .any(|a| a.id == local && a.domain.id() == module.id())
                    })
                })
            })
    }

    /// The audit rules registered here (the extension running, `audit.provider` granted, as an agent's `audits` lists
    /// them) that join this program's audit, in the registry's order.
    pub(crate) fn ext_audit_choices(&self) -> Vec<RuleChoice> {
        // The open project's module's rules, in kentos-cad too (docs/adr/9195 §2).
        let (Some(registry), Some(program)) = (self.extensions.registry.as_ref(), self.active_module())
        else {
            return Vec::new();
        };
        registry
            .contributions()
            .iter()
            .filter(|c| c.kind == ContributionKind::Audit)
            .filter_map(|c| {
                let ext = c.lease.extension.clone();
                let local = c.id.strip_prefix(&format!("ext.{ext}."))?.to_owned();
                let joins = registry
                    .manifest(&ext)?
                    .entrypoints
                    .audits
                    .iter()
                    .any(|a| a.id == local && a.domain.id() == program.id());
                joins.then(|| RuleChoice {
                    words: format!("{} › {}", self.extension_name(&ext), c.title),
                    ext,
                    local,
                })
            })
            .collect()
    }

    /// The rule the picker stands on: the one chosen since the last run while it is still offered, else the last
    /// run's, else the first.
    pub(super) fn ext_audit_picked(&self, rules: &[RuleChoice]) -> Option<usize> {
        let at =
            |ext: &str, local: &str| rules.iter().position(|r| r.ext == ext && r.local == local);
        self.extensions
            .audit_pick
            .as_ref()
            .and_then(|(e, l)| at(e, l))
            .or_else(|| {
                self.extensions
                    .audit
                    .as_ref()
                    .and_then(|a| at(&a.ext, &a.local))
            })
            .or((!rules.is_empty()).then_some(0))
    }

    /// Kural: [`rules`] Denetle; none when no extension here has a rule for this program.
    pub(crate) fn ext_audit_picker(&self, rules: &[RuleChoice]) -> Option<Element<'_, Message>> {
        let picked = self.ext_audit_picked(rules)?;
        let running = self
            .extensions
            .audit
            .as_ref()
            .is_some_and(|a| a.running.is_some());
        let chosen = rules[picked].clone();
        let select = Select::new(
            rules.iter().map(|r| Choice::new(r.words.clone())),
            Some(picked),
            |i| msg(Event::AuditPick(i)),
        );
        let run = words::primary(
            "Denetle",
            (!running).then(|| msg(Event::Run(chosen.ext, "audit".to_owned(), chosen.local))),
        );
        Some(
            row![label::body("Kural"), select, run]
                .spacing(10)
                .align_y(Center)
                .into(),
        )
    }

    /// The picker's choice, by its place in [`App::ext_audit_choices`].
    pub(crate) fn ext_audit_pick(&mut self, at: usize) {
        if let Some(r) = self.ext_audit_choices().into_iter().nth(at) {
            self.extensions.audit_pick = Some((r.ext, r.local));
        }
    }
}
