//! Room source scope beside the existing explicit architectural target level.
use crate::{tool::OptionChoice, tool_settings::Setting};
use kentos_commands::{ArchInfo, ProjectSettings, SpaceScope};

#[derive(Clone, Debug, Default)]
pub(crate) struct Choice {
    level: crate::architecture_level::Choice,
    unassigned: bool,
}
impl Choice {
    pub fn see(&mut self, settings: &ProjectSettings) { self.level.see(settings); }
    pub fn id(&self) -> Option<String> { self.level.id() }
    pub fn arch(&self) -> Option<ArchInfo> { self.level.arch() }
    pub fn scope(&self) -> SpaceScope {
        if self.unassigned { SpaceScope::Unassigned } else { SpaceScope::Project }
    }
    pub fn label(&self) -> String {
        if self.unassigned { "Yalnız atanmamış".into() }
        else if self.level.id().is_none() { "Tüm katlar".into() }
        else { self.level.label() }
    }
    pub fn setting(&self) -> Setting {
        let mut setting = self.level.setting();
        setting.choices[0].label="Tüm katlar".into();
        setting.choices[0].checked=self.level.id().is_none() && !self.unassigned;
        setting.choices.insert(1,OptionChoice {typed:"unassigned".into(),label:"Yalnız atanmamış".into(),
            checked:self.unassigned,icon:None,preview:None,command:None});
        setting.value=if self.unassigned { "Atanmamış".into() } else { self.label() };
        setting
    }
    pub fn choose(&mut self, value: &str) -> bool {
        if value=="unassigned" {
            self.level.choose("none"); self.unassigned=true; true
        } else if self.level.choose(value) {
            self.unassigned=false; true
        } else { false }
    }
    pub fn next(&mut self) {
        if self.unassigned { self.unassigned=false; }
        else {
            self.level.next();
            self.unassigned=self.level.id().is_none();
        }
    }
}
