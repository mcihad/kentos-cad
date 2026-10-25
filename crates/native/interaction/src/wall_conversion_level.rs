//! Explicit conversion target level; no guessing from source CAD attributes.
use crate::{tool::OptionChoice, tool_settings::Setting};
use kentos_commands::{ArchInfo, ArchLevel, ProjectSettings};

#[derive(Default)]
pub(super) struct Choice {
    selected: Option<String>,
    levels: Vec<ArchLevel>,
}

impl Choice {
    pub fn see(&mut self, settings: &ProjectSettings) {
        self.levels = settings
            .arch
            .as_ref()
            .map_or_else(Vec::new, |a| a.levels.clone());
        // A removed selection remains explicit and fails command validation.
        // Silently reverting it would write walls to the wrong level.
    }
    pub fn id(&self) -> Option<String> {
        self.selected.clone()
    }
    pub fn arch(&self) -> Option<ArchInfo> {
        self.selected.as_ref().map(|id| ArchInfo {
            level: Some(id.clone()),
            ..Default::default()
        })
    }
    pub fn label(&self) -> String {
        match &self.selected {
            None => "Belirtilmedi".into(),
            Some(id) => self
                .levels
                .iter()
                .find(|l| l.id == *id)
                .map_or_else(|| format!("Bulunamadı: {id}"), |l| l.name.clone()),
        }
    }
    pub fn setting(&self) -> Setting {
        let option = |typed: String, label: String, checked| OptionChoice {
            typed,
            label,
            checked,
            icon: None,
            preview: None,
            command: None,
        };
        let mut choices = vec![option(
            "none".into(),
            "Belirtilmedi".into(),
            self.selected.is_none(),
        )];
        choices.extend(self.levels.iter().map(|l| {
            option(
                format!("id:{}", l.id),
                l.name.clone(),
                self.selected.as_ref() == Some(&l.id),
            )
        }));
        if let Some(id) = &self.selected
            && !self.levels.iter().any(|l| l.id == *id)
        {
            choices.push(option(format!("id:{id}"), self.label(), true));
        }
        choices.push(OptionChoice {
            command: Some("arch.settings"),
            ..option("edit".into(), "Katları düzenle…".into(), false)
        });
        Setting::choice("L", "Kat", choices)
    }
    pub fn choose(&mut self, value: &str) -> bool {
        if value == "none" {
            self.selected = None;
        } else if let Some(id) = value.strip_prefix("id:")
            && self.levels.iter().any(|l| l.id == id)
        {
            self.selected = Some(id.into());
        } else {
            return false;
        }
        true
    }
    pub fn next(&mut self) {
        self.selected = match self
            .selected
            .as_ref()
            .and_then(|id| self.levels.iter().position(|l| l.id == *id))
        {
            Some(i) => self.levels.get(i + 1).map(|l| l.id.clone()),
            None => self.levels.first().map(|l| l.id.clone()),
        };
    }
}
