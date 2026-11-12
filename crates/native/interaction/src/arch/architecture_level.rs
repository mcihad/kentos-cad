//! Explicit architectural target level; no guessing from source CAD attributes. Until one is chosen here it follows
//! Etkin kat (docs/adr/0713), the host's level for new objects.
use crate::{tool::OptionChoice, tool_settings::Setting};
use kentos_commands::{ArchInfo, ArchLevel, Entity, ProjectSettings};
use kentos_domain::Document;

#[derive(Clone, Debug, Default)]
pub(crate) struct Choice {
    selected: Option<String>,
    levels: Vec<ArchLevel>,
    /// A level (or none) was chosen here: Etkin kat is no longer followed.
    chosen: bool,
    /// Whether the drawing has walls on a level, as last asked: its revision, the level and the answer.
    walls: Option<(u64, String, bool)>,
}

/// Whether the drawing has a wall on level `id`.
fn walls_on(doc: &Document, id: &str) -> bool {
    doc.entities()
        .any(|e| matches!(e, Entity::Wall(w) if w.wall.arch.level.as_deref() == Some(id)))
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
    /// Follows `active`, Etkin kat (docs/adr/0713), until a level is chosen here; one not of the drawing is none.
    pub fn follow(&mut self, active: Option<&str>) {
        if !self.chosen {
            self.selected = active
                .filter(|id| self.levels.iter().any(|l| l.id == *id))
                .map(str::to_owned);
        }
    }

    /// `follow`, for a tool that looks for walls (a room's, a cut's): Etkin kat while the drawing has walls on it,
    /// else every level, as before levels were set (an older drawing's walls have none).
    pub fn follow_where_walls(&mut self, active: Option<&str>, doc: &Document) {
        let active = active.filter(|id| {
            let revision = doc.revision();
            match &self.walls {
                Some((at, level, yes)) if *at == revision && level == id => *yes,
                _ => {
                    let yes = walls_on(doc, id);
                    self.walls = Some((revision, (*id).to_owned(), yes));
                    yes
                }
            }
        });
        self.follow(active);
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
                .map_or_else(
                    || format!("Bulunamadı: {id}"),
                    // Followed, not chosen: said so (docs/adr/0713).
                    |l| {
                        if self.chosen {
                            l.name.clone()
                        } else {
                            format!("{} (etkin kat)", l.name)
                        }
                    },
                ),
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
        self.chosen = true;
        true
    }
    pub fn next(&mut self) {
        self.chosen = true;
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
