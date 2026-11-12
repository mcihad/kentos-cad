//! The Denetim tab's writes (arch_audit.rs): Düzelt as the one fix's call or one change set of the chosen ones, the
//! exceptions through `cad.arch.exceptions`, Kata ver through `cad.levels.assign`; each one undo step, the drawing
//! checked again after it (docs/adr/0703, 0717, 0732).

use kentos_commands::CallOp;
use kentos_domain::{Slot, Uuid};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, registry};

use super::STALE;
use crate::app::App;

impl App {
    /// The chosen findings' fixes as one command call: the one fix's own, or a change set of them in the findings'
    /// order (one undo step, “Denetim düzeltmeleri”; docs/adr/0732 §5).
    pub(super) fn arch_audit_fix_call(&self) -> Option<(String, serde_json::Value, String)> {
        let fixes: Vec<&kentos_commands::ArchAuditFix> =
            self.arch_audit_chosen().into_iter().filter_map(|f| f.fix.as_ref()).collect();
        match fixes.as_slice() {
            [] => None,
            [one] => Some((one.step.command.clone(), one.step.input.clone(), one.label.clone())),
            many => {
                let steps: Vec<serde_json::Value> = many
                    .iter()
                    .map(|f| serde_json::json!({"command": f.step.command, "input": f.step.input, "name": f.label}))
                    .collect();
                Some((
                    kentos_commands::CAD_CHANGESET_APPLY.to_owned(),
                    serde_json::json!({"steps": steps, "label": "Denetim düzeltmeleri"}),
                    format!("{} düzeltme", many.len()),
                ))
            }
        }
    }

    /// Düzelt: the chosen findings' fixes, as one undo step; the drawing checked again.
    pub(super) fn arch_audit_fix(&mut self) {
        if self.arch_audit_stale() {
            self.warn(STALE);
            return;
        }
        let Some((command, input, label)) = self.arch_audit_fix_call() else {
            return;
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let answer = registry::run(
            &mut ExecutionContext::new(&mut doc.model),
            &command,
            CallOp::Execute,
            input,
        );
        match answer {
            Ok(v) if v["status"] == "completed" => {
                self.arch_audit.chosen.clear();
                self.say(Level::Success, format!("Düzeltildi: {label}. Geri al ile dönülür."));
                for w in v["warnings"].as_array().into_iter().flatten() {
                    if let Some(m) = w["message"].as_str() {
                        self.warn(m.to_owned());
                    }
                }
            }
            Ok(v) => {
                let why = v["error"]["message"].as_str().unwrap_or("Düzeltme yazılamadı.");
                self.warn(why.to_owned());
                return;
            }
            Err(e) => {
                self.warn(format!("Düzeltme çalıştırılamadı: {e:?}"));
                return;
            }
        }
        self.arch_audit_check();
    }

    /// İstisna yap's note given: the chosen findings that are no exceptions yet marked through `cad.arch.exceptions`,
    /// with the note when one is written; one undo step; the drawing checked again (docs/adr/0732 §3).
    pub(super) fn arch_audit_except(&mut self) {
        let Some(note) = self.arch_audit.note.take() else {
            return;
        };
        if self.arch_audit_stale() {
            self.warn(STALE);
            return;
        }
        let keys: Vec<String> = self
            .arch_audit_chosen()
            .into_iter()
            .filter(|f| !f.exception)
            .map(|f| f.key.clone())
            .collect();
        if keys.is_empty() {
            return;
        }
        let mut input = serde_json::json!({
            "operation": "mark",
            "keys": keys,
            "audit": self.arch_audit_finding_input(),
        });
        let note = note.trim();
        if !note.is_empty() {
            input["note"] = note.into();
        }
        self.arch_audit_exceptions(input, |n| match n {
            0 => "İstisnalar zaten yazılıydı.".to_owned(),
            n => format!("{n} bulgu istisna oldu; listede gizlenir. Geri al ile dönülür."),
        });
    }

    /// İstisnayı kaldır: the chosen findings' exceptions, lapsed or not, taken away; one undo step.
    pub(super) fn arch_audit_unexcept(&mut self) {
        let keys: Vec<String> = self
            .arch_audit_chosen()
            .into_iter()
            .filter(|f| f.exception || f.lapsed)
            .map(|f| f.key.clone())
            .collect();
        if keys.is_empty() {
            return;
        }
        self.arch_audit_exceptions(serde_json::json!({ "operation": "unmark", "keys": keys }), |n| {
            format!("{n} istisna kaldırıldı; bulgular yeniden açık. Geri al ile dönülür.")
        });
    }

    /// Eski istisnaları temizle: the exceptions whose finding the bar's check no longer finds dropped; one undo step.
    pub(super) fn arch_audit_prune(&mut self) {
        let input = serde_json::json!({ "operation": "prune", "audit": self.arch_audit_finding_input() });
        self.arch_audit_exceptions(input, |n| match n {
            0 => "Temizlenecek eski istisna yok.".to_owned(),
            n => format!("Bulgusu kalmamış {n} istisna silindi. Geri al ile dönülür."),
        });
    }

    /// `cad.arch.exceptions` with `input`, its answer said by `said` from how many exceptions it changed; the drawing
    /// checked again.
    fn arch_audit_exceptions(&mut self, input: serde_json::Value, said: impl Fn(usize) -> String) {
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let answer = registry::run(
            &mut ExecutionContext::new(&mut doc.model),
            kentos_commands::CAD_ARCH_EXCEPTIONS,
            CallOp::Execute,
            input,
        );
        match answer {
            Ok(v) if v["status"] == "completed" => {
                let n = v["output"]["changed"].as_array().map_or(0, Vec::len);
                self.say(Level::Success, said(n));
            }
            Ok(v) => {
                let why = v["error"]["message"].as_str().unwrap_or("İstisna yazılamadı.");
                self.warn(why.to_owned());
                return;
            }
            Err(e) => {
                self.warn(format!("İstisna yazılamadı: {e:?}"));
                return;
            }
        }
        self.arch_audit.chosen.clear();
        self.arch_audit_check();
    }

    /// Kata ver ▾'s Seç ve göster: the objects without a level selected and the view on them, before any level is
    /// given (docs/adr/0717 §2).
    pub(super) fn arch_audit_show_levelless(&mut self) {
        let uids = self.arch_audit_levelless();
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        let slots: Vec<Slot> = uids
            .iter()
            .filter_map(|u| Uuid::parse_str(u).ok())
            .filter_map(|u| doc.model.slot_of(u))
            .collect();
        let ids: Vec<f64> = slots.iter().map(|s| f64::from(s.0)).collect();
        self.selection.set(slots);
        if !ids.is_empty() {
            self.navigating(|app| app.zoom_to_objects(&ids));
        }
    }

    /// Kata ver ▾'s level: every object without a level given it through `cad.levels.assign`, one undo step; the
    /// objects stay selected so the step shows; the drawing checked again.
    pub(super) fn arch_audit_assign_level(&mut self, level: &str) {
        if self.arch_audit_stale() {
            self.warn(STALE);
            return;
        }
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let answer = registry::run(
            &mut ExecutionContext::new(&mut doc.model),
            kentos_commands::CAD_LEVELS_ASSIGN,
            CallOp::Execute,
            serde_json::json!({ "level": level }),
        );
        match answer {
            Ok(v) if v["status"] == "completed" => {
                let given: Vec<Slot> = v["output"]["assigned"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|a| a["id"].as_u64())
                    .map(|id| Slot(id as u32))
                    .collect();
                let name = doc
                    .model
                    .settings()
                    .arch
                    .as_ref()
                    .and_then(|a| a.levels.iter().find(|l| l.id == level))
                    .map_or_else(|| level.to_owned(), |l| l.name.clone());
                let count = given.len();
                self.selection.set(given);
                self.say(
                    Level::Success,
                    format!("{count} nesnenin katı {name} oldu. Geri al ile dönülür."),
                );
                for w in v["warnings"].as_array().into_iter().flatten() {
                    if let Some(m) = w["message"].as_str() {
                        self.warn(m.to_owned());
                    }
                }
            }
            Ok(v) => {
                let why = v["error"]["message"].as_str().unwrap_or("Kat verilemedi.");
                self.warn(why.to_owned());
                return;
            }
            Err(e) => {
                self.warn(format!("Kat verilemedi: {e:?}"));
                return;
            }
        }
        self.arch_audit_check();
    }
}
