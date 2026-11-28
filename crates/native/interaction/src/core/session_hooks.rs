//! What the host asks of the running tool through the session, without a pointer or a key (split from session.rs):
//! its option's values, its settings and those the value field offers, a change of the drawing, and İşlemler ›
//! Geçmiş's Yeniden aç on a run it recorded (docs/adr/9334).

use crate::session::Session;
use crate::tool::Context;

impl Session {
    /// The values the running tool's option `key` chooses between (Yazı's
    /// Hiza, docs/adr/0145 §6); none without a tool or such an option.
    pub fn option_choices(&self, key: &str) -> Vec<crate::tool::OptionChoice> {
        self.tool
            .as_ref()
            .map_or_else(Vec::new, |tool| tool.option_choices(key))
    }

    pub fn settings(&self, format: &crate::Format) -> Vec<crate::tool_settings::Setting> {
        self.tool
            .as_ref()
            .map_or_else(Vec::new, |tool| tool.settings(format))
    }

    /// İşlemler › Geçmiş's Yeniden aç (docs/adr/9334): the running tool takes a run's values back; false when it
    /// records no runs.
    pub fn reopen(
        &mut self,
        values: &std::collections::BTreeMap<String, serde_json::Value>,
        cx: &mut Context<'_>,
    ) -> bool {
        self.tool
            .as_mut()
            .is_some_and(|tool| tool.reopen(values, cx))
    }

    /// Refresh decorations after a model change, even with the pointer in
    /// a panel. Tools that do not cache document-derived previews opt out.
    pub fn document_changed(&mut self, cx: &Context<'_>) {
        if let Some(tool) = self.tool.as_mut() {
            tool.document_changed(cx);
        }
    }
    /// The running tool's settings the value field offers after its locks, in Tab's order (docs/adr/8018).
    pub fn cursor_settings(&self, format: &crate::Format) -> Vec<crate::tool_settings::Setting> {
        let Some(tool) = self.tool.as_ref() else {
            return Vec::new();
        };
        let rows = tool.settings(format);
        tool.cursor_settings()
            .iter()
            .filter_map(|key| rows.iter().find(|r| r.key == *key && r.editable).cloned())
            .collect()
    }
}
